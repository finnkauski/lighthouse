pub mod helpers {
    // imports
    use std::net::IpAddr;
    use url::Url;

    /// Generates the target URL for the bridge
    pub fn generate_target(address: IpAddr, token: &str) -> Result<Url, ()> {
        let mut target = Url::parse("http://localhost").unwrap(); // Unwrap as it can't fail in parsing
        let path = format!("api/{}/", token);
        target.set_path(&path[..]);
        if target.set_ip_host(address).is_ok() {
            return Ok(target);
        }
        Err(())
    }

    pub mod network {
        use crate::lights::SendableState;
        use url::Url;

        /// Defines the allowed methods to be sent to bridge
        pub enum AllowedMethod {
            GET,
            PUT,
            POST,
        }

        /// Implementated to allow controlled conversion into reqwest
        /// methods and not allow undefined methods to be sent to bridge
        impl std::convert::From<AllowedMethod> for reqwest::Method {
            fn from(value: AllowedMethod) -> Self {
                match value {
                    AllowedMethod::GET => reqwest::Method::GET,
                    AllowedMethod::POST => reqwest::Method::POST,
                    AllowedMethod::PUT => reqwest::Method::PUT,
                }
            }
        }

        /// This type alias is a URL and a type of Request to be sent
        pub type RequestTarget = (Url, AllowedMethod);

        /// This alias allows us to store multiple targeted lights
        type RequestTargets = Vec<RequestTarget>;

        /// NewStates is a convenience alias allowing us to store
        /// optional vectors of states that we want to send if we
        /// are using PUT or POST to a given endpoint
        pub type NewStates<'a> = Vec<Option<&'a SendableState>>;

        /// Convenience type alias for a possible Result from the reqwest client
        type ResponseResult = Result<reqwest::Response, reqwest::Error>;
        type IndexedResponseResult = (usize, ResponseResult);

        /// Function wrapping the request sending functionality
        /// to a location.
        pub async fn send_request(
            request_target: RequestTarget,
            state: Option<&SendableState>,
            client: &reqwest::Client,
        ) -> ResponseResult {
            let (target, method) = request_target;
            match method {
                AllowedMethod::POST => client.post(target).json(&state).send().await,
                AllowedMethod::GET => client.get(target).send().await,
                AllowedMethod::PUT => client.put(target).json(&state).send().await,
            }
        }

        pub async fn send_request_indexed(
            index: usize,
            request_target: RequestTarget,
            state: Option<&SendableState>,
            client: &reqwest::Client,
        ) -> IndexedResponseResult {
            (index, send_request(request_target, state, client).await)
        }

        /// Function that sends off several states to the lights
        /// This is much more key than individual requests functionality provided by the
        /// send_request function as this is allowing us to do this asynchronously across
        /// an arbitrary selection of lights.
        pub async fn send_requests(
            request_targets: RequestTargets,
            states: NewStates<'_>,
            client: &reqwest::Client,
        ) -> Vec<ResponseResult> {
            use tokio::stream::StreamExt;
            let mut f: futures::stream::FuturesUnordered<_> = request_targets
                .into_iter()
                .zip(states.into_iter())
                .enumerate()
                .map(|(i, (target, state))| send_request_indexed(i, target, state, client))
                .collect();
            let mut res = Vec::with_capacity(f.len());
            while let Some(tup) = f.next().await {
                res.push(tup);
            }
            res.sort_by_key(|tuple| tuple.0);
            res.into_iter().map(|tup| tup.1).collect()
        }
    }
}

pub mod bridge {
    // imports
    use super::{
        helpers::{network::*, *},
        lights::*,
    };
    use std::collections::BTreeMap;
    use std::net::IpAddr;
    use tokio::runtime::Runtime;
    use url::Url;

    #[derive(Debug)]
    pub struct Bridge {
        pub target: Url,
        client: reqwest::Client,
        runtime: Runtime,
    }

    /// # Take it to the Bridge!
    ///
    /// This is the Bridge object - the core of the library.
    ///
    /// This is the manager struct that implements all the core methods that are required to interact
    /// with the HueBridge. It has a bunch of convenience functions such as sending state, scanning for lights,
    /// getting the information about your existing lights and finding bridge IP addresses with SSDP.
    impl Bridge {
        /// Constructor for a bridge from and IP and a token
        pub fn new(ip: IpAddr, token: &str) -> Result<Self, ()> {
            let target = generate_target(ip, token)?;
            let client = reqwest::Client::new();
            let runtime = tokio::runtime::Builder::new()
                .basic_scheduler()
                .enable_all()
                .build()
                .expect("Could not create tokio runtime during the creation of the Bridge");
            Ok(Bridge {
                target,
                client,
                runtime,
            })
        }

        pub fn scan(&mut self) -> BTreeMap<u8, Light> {
            let endpoint = self.get_endpoint("./lights", AllowedMethod::GET);
            let fut = send_request(endpoint, None, &self.client);
            let lights: BTreeMap<u8, Light> = self
                .runtime
                .block_on(async { fut.await?.json().await })
                .expect("Could not completely decode/send request");
            lights
        }

        /// Sends a state to a given light by its ID on the system.
        ///
        /// This is useful when you want to send a given state to one light
        /// on the network.
        pub fn state_to(&mut self, id: u8, new_state: &SendableState) -> reqwest::Response {
            let endpoint =
                self.get_endpoint(&format!("./lights/{}/state", id)[..], AllowedMethod::PUT);
            self.runtime
                .block_on(send_request(endpoint, Some(new_state), &self.client))
                .expect(&format!("Could not send state to light: {}", id)[..])
        }

        /// Send a state object to all lights on the network.
        pub fn state_to_multiple(
            &mut self,
            ids: impl IntoIterator<Item = u8>,
            new_states: Vec<&SendableState>,
        ) -> Result<Vec<reqwest::Response>, reqwest::Error> {
            let endpoints = ids
                .into_iter()
                .map(|id| {
                    self.get_endpoint(&format!("./lights/{}/state", id)[..], AllowedMethod::PUT)
                })
                .collect();
            let states = new_states.into_iter().map(Some).collect();

            self.runtime
                .block_on(send_requests(endpoints, states, &self.client))
                .into_iter()
                .collect()
        }

        /// Provided an endpoint string, and a method it will create a `RequestTarget` that can
        /// be sent a request. The final URI will depend on the `self.target` field and the string
        /// provided.
        fn get_endpoint(&self, s: &str, method: AllowedMethod) -> RequestTarget {
            (self.target.join(s).unwrap(), method)
        }

        /// Method to find bridge IP addressed on the network.
        ///
        /// If multiple are found, they are all returned.
        pub fn find_bridges() -> Vec<IpAddr> {
            use ssdp::header::{HeaderMut, Man, MX, ST};
            use ssdp::message::{Multicast, SearchRequest};

            println!("Searching for bridges (5s)...");
            // create request with required headers for the sddp search
            let mut request = SearchRequest::new();
            request.set(Man);
            request.set(MX(5));
            request.set(ST::Target(ssdp::FieldMap::URN(
                "urn:schemas-upnp-org:device:Basic:1".into(),
            )));

            // Find devices
            let devices = request
                .multicast()
                .expect("Could not perform multicast request");

            // Coerce this into a vector
            let mut result: Vec<IpAddr> = devices.into_iter().map(|(_, src)| src.ip()).collect();

            result.sort(); // TODO: see if this is necessary
            result.dedup();

            result
        }

        /// Print useful information about the state of your system
        ///
        /// Namely, asks for all available lights and print a JSON representation
        /// of the system to STDOUT.
        pub fn system_info(&mut self) {
            let fut = send_request(
                self.get_endpoint("./lights", AllowedMethod::GET),
                None,
                &self.client,
            );
            match self
                .runtime
                .block_on(async { fut.await.expect("Could not perform request").json().await })
            {
                Ok(resp) => {
                    let r: serde_json::Value = resp;
                    println!("{}", serde_json::to_string_pretty(&r).unwrap());
                }
                Err(e) => {
                    println!("Could not send the get request: {}", e);
                }
            };
        }

        /// Conditional feature:
        ///
        /// If `persist` feature is enabled, then this allows creating a bridge from environment
        /// variables.
        ///
        /// The variables that will be looked up are:
        /// - HUE_BRIDGE_IP - the IP of the bridge on the local network
        /// - HUE_BRIDGE_KEY - the KEY that you get when you register to the bridge.
        #[cfg(feature = "persist")]
        pub fn from_env() -> Bridge {
            let ip = std::env::var("HUE_BRIDGE_IP")
                .expect("Could not find HUE_BRIDGE_IP environment variable.")
                .parse()
                .expect("Could not parse the address in the variable: HUE_BRIDGE_IP.");
            let key = std::env::var("HUE_BRIDGE_KEY")
                .expect("Could not find HUE_BRIDGE_KEY environment variable.");

            Bridge::new(ip, &key).expect(&format!("Could not create new bridge (IP {})", ip)[..])
        }

        /// Conditional feature:
        ///
        /// If one wants to use `ron` to serialise the Bridge, this provides a method to save the bridge
        /// to a text file using `ron`.
        ///
        /// Note: Enable the `ron` feature.
        #[cfg(feature = "persist")]
        pub fn to_ron(&self, filename: &str) {
            todo!()
        }

        /// Conditional feature:
        ///
        /// Allows loading a Bridge from a `ron` file.
        ///
        /// Note: Enable the `ron` feature.
        #[cfg(feature = "persist")]
        pub fn from_ron(&self, _filename: &str) {
            todo!()
        }
    }
}

pub mod lights {
    // imports
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    /// Struct that can be sent to the Hue lights. It mirrors closely the
    /// `State`.
    ///
    /// ```
    /// use lighthouse::{lights::*, state};
    /// let state_1: SendableState = serde_json::from_str(r#"{"on":true}"#).unwrap();
    /// let state_2: SendableState = SendableState {on:Some(true), ..SendableState::default()};
    /// let state_3: &SendableState = state!(on: true, xy: [1.0, 0.123]);
    /// let state_4: SendableState = state!(nonref; on: true, xy: [1.0, 0.123]);
    /// ```
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct SendableState {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub on: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bri: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub hue: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sat: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub effect: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub xy: Option<[f32; 2]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub alert: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub transitiontime: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub colormode: Option<String>,
    }

    /// This would be good to reimplement in some way to respect the current state of a given light.
    /// TODO: Add a - delta state type which is just state change
    impl Default for SendableState {
        /// Initialises the state as Nones
        fn default() -> Self {
            Self {
                on: None,
                bri: None,
                hue: None,
                sat: None,
                effect: None,
                xy: None,
                alert: None,
                transitiontime: Some(1),
                colormode: None,
            }
        }
    }

    /// This object contains the state part  of each light
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct State {
        pub on: bool,
        pub bri: u8,
        pub hue: u16,
        pub sat: u8,
        pub effect: String,
        pub xy: [f32; 2],
        pub ct: u32,
        pub alert: String,
        pub colormode: String,
        pub mode: String,
        pub reachable: bool,
    }

    impl From<State> for SendableState {
        fn from(state: State) -> Self {
            Self {
                on: Some(state.on),
                bri: Some(state.bri),
                hue: Some(state.hue),
                sat: Some(state.sat),
                effect: Some(state.effect),
                xy: Some(state.xy),
                alert: None,
                transitiontime: Some(1),
                colormode: Some(state.colormode),
            }
        }
    }

    impl std::fmt::Display for Light {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{{ on: {} }} : {}", self.state.on, self.name)
        }
    }

    /// Light object representing the complete state of a light
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Light {
        pub state: State,
        pub swupdate: Value,
        pub r#type: String,
        pub name: String,
        pub modelid: String,
        pub manufacturername: String,
        pub productname: String,
        pub capabilities: Value,
        pub config: Value,
        pub uniqueid: String,
        pub swversion: String,
        pub swconfigid: String,
        pub productid: String,
    }

    /// Super useful macro to create `SendableState`
    /// ```
    /// use lighthouse::{lights::*, state};
    /// // Usage examples
    /// // Returns a reference, most useful as a default
    /// let sendable_state: &SendableState = state!(on: true, xy: [1.0, 0.0]);
    ///
    /// // Returns a value, still useful.
    /// let sendable_state: SendableState = state!(nonref; on: true, xy: [1.0, 0.0]);
    /// ```
    #[macro_export]
    macro_rules! state {
    ($($i:ident:$v:expr), *) => {
        &lighthouse::lights::SendableState {
            $($i: Some($v),) *
            ..lighthouse::lights::SendableState::default()
        }
    };

    (nonref; $($i:ident:$v:expr),*) => {
        lighthouse::lights::SendableState {
            $($i: Some($v),)*
            ..lighthouse::lights::SendableState::default()
        }
    };

    (from: $state:expr; $($i:ident:$v:expr),*) => {{
        let mut sendable = lighthouse::lights::SendableState::from($state);
        $(sendable.$i = Some($v);)*
        sendable
    }};


}
}

mod tests {
    mod helpers {
        #[test]
        fn test_generate_target() {
            use std::net::IpAddr;
            use url::Url;

            let addr: IpAddr = "192.168.0.1".parse().unwrap();
            assert_eq!(
                crate::helpers::generate_target(addr, "tokengoeshere1234").unwrap(),
                Url::parse("http://192.168.0.1/api/tokengoeshere1234/").unwrap()
            )
        }

        #[test]
        fn test_httpbin_get() {
            use crate::helpers::network::{run_future, send_requests, AllowedMethod};
            let targets = vec![
                (
                    url::Url::parse("https://httpbin.org/get").unwrap(),
                    AllowedMethod::GET,
                ),
                (
                    url::Url::parse("https://httpbin.org/post").unwrap(),
                    AllowedMethod::POST,
                ),
                (
                    url::Url::parse("https://httpbin.org/put").unwrap(),
                    AllowedMethod::PUT,
                ),
            ];
            let states = vec![None, None, None];
            let client = reqwest::Client::new();
            let correct: bool = run_future(send_requests(targets, states, &client))
                .into_iter()
                .all(|r| r.unwrap().status() == 200);
            assert!(correct);
        }
    }
}
