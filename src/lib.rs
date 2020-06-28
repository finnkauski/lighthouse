pub mod helpers {
    // imports
    use std::net::IpAddr;
    use url::Url;

    /// Generates the target URL for the bridge
    pub fn generate_target(address: IpAddr, token: &str) -> Result<Url, String> {
        let mut target = Url::parse("http://localhost").unwrap(); // Unwrap as it can't fail in parsing
        let path = format!("api/{}/", token);
        target.set_path(&path[..]);
        match target.set_ip_host(address) {
            Ok(_) => Ok(target),
            Err(_) => Err("Could not set the ip address as the url".into()),
        }
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
        type NewStates<'a> = Vec<Option<&'a SendableState>>;

        /// Convenience type alias for a possible Result from the reqwest client
        type ResponseResult = Result<reqwest::Response, reqwest::Error>;

        /// Function wrapping the request sending functionality
        /// to a location.
        pub async fn send_request(
            request_target: RequestTarget,
            state: Option<&SendableState>,
        ) -> ResponseResult {
            let client = reqwest::Client::new();
            let (target, method) = request_target;
            match method {
                AllowedMethod::POST => client.post(target).json(&state).send().await,
                AllowedMethod::GET => client.get(target).send().await,
                AllowedMethod::PUT => client.put(target).json(&state).send().await,
            }
        }

        /// Function that sends off several states to the lights
        /// This is much more key than individual requests functionality provided by the
        /// send_request function as this is allowing us to do this asynchronously across
        /// an arbitrary selection of lights.
        pub async fn send_requests(
            request_targets: RequestTargets,
            states: NewStates<'_>,
        ) -> Vec<ResponseResult> {
            futures::future::join_all(
                request_targets
                    .into_iter()
                    .zip(states.into_iter())
                    .map(|(target, state)| send_request(target, state)),
            )
            .await
        }

        #[tokio::main]
        pub async fn run_future<T>(fut: impl std::future::Future<Output = T>) -> T {
            fut.await
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
    use url::Url;

    #[derive(Debug)]
    pub struct Bridge {
        pub target: Url,
    }

    impl Bridge {
        /// Constructor for a bridge from and IP and a token
        pub fn new(ip: IpAddr, token: &str) -> Result<Self, String> {
            let target = generate_target(ip, token)?;
            Ok(Bridge { target })
        }

        pub fn scan(&mut self) -> BTreeMap<u8, Light> {
            let endpoint = self.get_endpoint("./lights", AllowedMethod::GET);
            let lights: BTreeMap<u8, Light> =
                run_future(async { send_request(endpoint, None).await?.json().await })
                    .expect("Could not completely decode/send request");
            lights
        }

        /// Send state to a given id
        pub fn state_to(&mut self, id: u8, new_state: &SendableState) -> reqwest::Response {
            let endpoint =
                self.get_endpoint(&format!("./lights/{}/state", id)[..], AllowedMethod::PUT);
            run_future(send_request(endpoint, Some(new_state)))
                .expect(&format!("Could not send state to light: {}", id)[..])
        }

        /// Get endpoint generated from the bridge target
        fn get_endpoint(&self, s: &str, method: AllowedMethod) -> RequestTarget {
            (self.target.join(s).unwrap(), method)
        }

        /// Method to find bridge ip addressed on the network
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
            let correct: bool = run_future(send_requests(targets, states))
                .into_iter()
                .all(|r| r.unwrap().status() == 200);
            assert!(correct);
        }
    }
}
