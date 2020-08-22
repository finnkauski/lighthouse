// TODO: Implement a Bridge Builder and move the building functions out of the actual bridge
// TODO: Add validation check for when making a bridge - ping some API endpoint to collect data. Good way to get more info as well about the bridge
/// # Helpers
///
/// The helpers module contains functions that assist the rest of the codebase
/// it is unlikely that any of these internals will have to be used manually. pub mod helpers {// imports use std::net::IpAddr; use url::Url; /// Generates the target URL for the bridge pub fn generate_target(address: IpAddr, token: &str) -> Result<Url, ()> {let mut target = Url::parse("http://localhost").unwrap(); // Unwrap as it can't fail in parsing let path = format!("api/{}/", token); target.set_path(&path[..]); if target.set_ip_host(address).is_ok() {return Ok(target);} Err(())} pub mod network {use crate::lights::SendableState; use url::Url; /// Defines the allowed methods to be sent to bridge pub enum AllowedMethod {GET, PUT, POST,} /// Implementated to allow controlled conversion into reqwest /// methods and not allow undefined methods to be sent to bridge impl std::convert::From<AllowedMethod> for reqwest::Method {fn from(value: AllowedMethod) -> Self {match value {AllowedMethod::GET => reqwest::Method::GET, AllowedMethod::POST => reqwest::Method::POST, AllowedMethod::PUT => reqwest::Method::PUT,}}} /// This type alias is a URL and a type of Request to be sent pub type RequestTarget = (Url, AllowedMethod); /// Convenience type alias for a possible Result from the reqwest client type ResponseResult = Result<reqwest::Response, reqwest::Error>; type IndexedResponseResult = (usize, ResponseResult); /// Function wrapping the request sending functionality /// to a location. pub async fn send_request(request_target: RequestTarget, state: Option<&SendableState>, client: &reqwest::Client,) -> ResponseResult {let (target, method) = request_target; match method {AllowedMethod::POST => client.post(target).json(&state).send().await, AllowedMethod::GET => client.get(target).send().await, AllowedMethod::PUT => client.put(target).json(&state).send().await,}} pub async fn send_request_indexed(index: usize, request_target: RequestTarget, state: Option<&SendableState>, client: &reqwest::Client,) -> IndexedResponseResult {(index, send_request(request_target, state, client).await)} /// Function that sends off several states to the lights /// This is much more key than individual requests functionality provided by the /// send_request function as this is allowing us to do this asynchronously across /// an arbitrary selection of lights. pub async fn send_requests(request_targets: impl IntoIterator<Item = RequestTarget>, states: impl IntoIterator<Item = Option<&SendableState>>, client: &reqwest::Client,) -> Vec<ResponseResult> {use tokio::stream::StreamExt; let mut f: futures::stream::FuturesUnordered<_> = request_targets .into_iter() .zip(states.into_iter()) .enumerate() .map(|(i, (target, state))| send_request_indexed(i, target, state, client)) .collect(); let mut res = Vec::with_capacity(f.len()); while let Some(tup) = f.next().await {res.push(tup);} res.sort_by_key(|tuple| tuple.0); res.into_iter().map(|tup| tup.1).collect()}}}
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
            request_targets: impl IntoIterator<Item = RequestTarget>,
            states: impl IntoIterator<Item = Option<&SendableState>>,
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
/// # Bridge module
///
/// This module contains the Bridge and related functionality
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

    /// # Take it to the Bridge!
    ///
    /// This is the Bridge object - the core of the library.
    ///
    /// This is the manager struct that implements all the core methods that are required to interact
    /// with the Hue Bridge. It has a bunch of convenience functions such as sending state, scanning for lights,
    /// getting the information about your existing lights and finding bridge IP addresses with SSDP.
    ///
    /// Additional features can be enabled:
    /// - `persist` - enables building a bridge from environment variables and serialising to file
    #[derive(Debug)]
    pub struct Bridge {
        pub target: Url,
        ip: IpAddr,
        token: String,
        client: reqwest::Client,
        runtime: Runtime,
    }

    impl Bridge {
        /// Constructor for a bridge from and IP and a token
        pub fn new(ip: IpAddr, token: String) -> Result<Self, ()> {
            let target = generate_target(ip, &token)?;
            let client = reqwest::Client::new();
            let runtime = tokio::runtime::Builder::new()
                .basic_scheduler()
                .enable_all()
                .build()
                .expect("Could not create tokio runtime during the creation of the Bridge");
            Ok(Bridge {
                target,
                ip,
                token,
                client,
                runtime,
            })
        }

        /// Scan the existing lights on the network. Returns the light id
        /// mapped to the light object.
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
        pub fn state_to_multiple<'a>(
            &mut self,
            ids: impl IntoIterator<Item = u8>,
            new_states: impl IntoIterator<Item = &'a SendableState>,
        ) -> Result<Vec<reqwest::Response>, reqwest::Error> {
            let endpoints: Vec<_> = ids
                .into_iter()
                .map(|id| {
                    self.get_endpoint(&format!("./lights/{}/state", id)[..], AllowedMethod::PUT)
                })
                .collect();
            let states = new_states.into_iter().map(Some);

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

        /// Method to interactively register a new bridge.
        ///
        /// This interacts with the user and guides them through the authentication flow to instantiate
        /// a new bridge.
        pub fn try_register(interactive: bool) -> Result<Self, String> {
            use serde_json::Value;

            let client = reqwest::Client::new();
            let mut runtime = tokio::runtime::Builder::new()
                .basic_scheduler()
                .enable_all()
                .build()
                .expect("Could not create tokio runtime during registration");

            let bridges = Self::find_bridges(); // find any bridges present on network

            if interactive {
                println!("Found the following bridges:\n{:#?}", bridges);
            }

            let body = serde_json::json!({ "devicetype": "lighthouse" });
            let mut check = |ip: IpAddr| -> Value {
                runtime.block_on(async {
                    client
                        .post(&format!("http://{}/api", ip))
                        .json(&body)
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap()
                })
            };

            let bridge_ip;
            let mut response;

            if bridges.is_empty() {
                return Err(String::from("Could not find any bridges on the network"));
            } else {
                if interactive {
                    println!(
                        "Will try to register. Please press the connection button on your bridge"
                    );
                }
                'wait_for_button: loop {
                    if interactive {
                        println!("Waiting for button press...");
                    }
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    for ip in &bridges {
                        response = check(*ip);
                        if response[0]["error"]["type"] == 101 {
                            continue;
                        } else {
                            bridge_ip = ip;
                            break 'wait_for_button;
                        }
                    }
                }
            }

            let token = response[0]["success"]["username"].to_string();
            let target = generate_target(*bridge_ip, &token)
                .expect("Could not create the required target after registration");

            Ok(Bridge {
                target,
                ip: *bridge_ip,
                token,
                client,
                runtime,
            })
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
                .expect("Could not find `HUE_BRIDGE_IP` environment variable.")
                .parse()
                .expect("Could not parse the address in the variable: HUE_BRIDGE_IP.");
            let key = std::env::var("HUE_BRIDGE_KEY")
                .expect("Could not find `HUE_BRIDGE_KEY` environment variable.");

            Bridge::new(ip, key).expect(&format!("Could not create new bridge (IP {})", ip)[..])
        }

        /// Conditional feature:
        ///
        /// Allows serializing the bridge to a text file. It is super simple
        /// and basically ends up writing out the bridge target to a text file.
        ///
        /// TODO: Move to using serde serialization
        #[cfg(feature = "persist")]
        pub fn to_file(&self, filename: &str) -> std::io::Result<()> {
            use std::io::prelude::*;
            let mut file = std::fs::File::create(filename)?;
            file.write_all(format!("{}\n{}", self.ip, self.token).as_ref())?;
            Ok(())
        }

        /// Conditional feature:
        ///
        /// Allows loading a Bridge from a text file.
        #[cfg(feature = "persist")]
        pub fn from_file(filename: &str) -> std::io::Result<Self> {
            use std::io::{BufRead, BufReader};
            let file = std::fs::File::open(filename)?;
            let reader = BufReader::new(file);

            let lines: Vec<String> = reader
                .lines()
                .enumerate()
                .map(|(idx, line)| line.unwrap_or_else(|_| format!("Could not read line {}", idx)))
                .collect();

            assert!(lines.len() == 2);
            Ok(Bridge::new(
                lines[0].parse().expect("Could not parse the provided IP"),
                lines[1].clone(),
            )
            .expect("Could not create Bridge"))
        }
    }
}

/// # Lights module
///
/// This module contains the core representations for the lights and responses
/// from the API.
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
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
        pub ct: Option<u32>,
        pub alert: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub colormode: Option<String>,
        pub mode: String,
        pub reachable: bool,
    }

    impl From<State> for SendableState {
        fn from(state: State) -> Self {
            Self {
                on: Some(state.on),
                bri: state.bri,
                hue: state.hue,
                sat: state.sat,
                effect: state.effect,
                xy: state.xy,
                alert: None,
                transitiontime: Some(1),
                colormode: state.colormode,
            }
        }
    }

    impl std::fmt::Display for Light {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match &*self {
                Self::LightBulb { state, name, .. } => {
                    write!(f, "{{ on: {} }} : {}", state.on, name)
                }
                Self::LightStrip { state, name, .. } => {
                    write!(f, "{{ on: {} }} : {}", state.on, name)
                }
            }
        }
    }

    /// Light enum representing the complete state of possible lights
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Light {
        LightBulb {
            state: State,
            swupdate: Value,
            r#type: String,
            name: String,
            modelid: String,
            manufacturername: String,
            productname: String,
            capabilities: Value,
            config: Value,
            uniqueid: String,
            swversion: String,
            swconfigid: String,
            productid: String,
        },
        LightStrip {
            state: State,
            swupdate: Value,
            r#type: String,
            name: String,
            modelid: String,
            manufacturername: String,
            productname: String,
            capabilities: Value,
            config: Value,
            uniqueid: String,
            swversion: String,
        },
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
        &$crate::lights::SendableState {
            $($i: Some($v),) *
            ..$crate::lights::SendableState::default()
        }
    };

    (nonref; $($i:ident:$v:expr),*) => {
        $crate::lights::SendableState {
            $($i: Some($v),)*
            ..$crate::lights::SendableState::default()
        }
    };

    (from: $state:expr; $($i:ident:$v:expr),*) => {{
        let mut sendable = $crate::lights::SendableState::from($state);
        $(sendable.$i = Some($v);)*
        sendable
    }};

}
}

/// # Color module (UNDERDEVELOPED)
///
/// This module (gated under the `color` feature) contains helpers in converting
/// colors to the required representations for the HUE API.
///
/// **NOTE:** Currently untested and work in progress. If you want to please submit
/// a PR with improvements.
#[cfg(feature = "color")]
pub mod color {
    use palette::{rgb::Srgb, Hsl};

    /// Convert from 'rgb' to the 'xy' values that can be sent to the
    /// hue lights. Does not internally use color gamut.
    ///
    /// **NOTE:** Currently no gamma correction is used. This was implemented based on the
    /// gist found [here](https://gist.github.com/popcorn245/30afa0f98eea1c2fd34d).
    pub fn rgb_to_xy(rgb: Vec<u8>) -> [f32; 2] {
        // NOTE: more information https://gist.github.com/popcorn245/30afa0f98eea1c2fd34d
        let standardise = |c: u8| {
            let val = (c as f32) / 255.0;
            if val > 0.04045 {
                ((val + 0.055) / (1.0 + 0.055)).powf(2.4)
            } else {
                val / 12.92
            }
        };

        let cnv: Vec<f32> = rgb.into_iter().map(standardise).collect();
        let (red, green, blue) = (cnv[0], cnv[1], cnv[2]);

        let x = red * 0.664_511 + green * 0.154_324 + blue * 0.162_028;
        let y = red * 0.283_881 + green * 0.668_433 + blue * 0.047_685;
        let z = red * 0.000_088 + green * 0.072_310 + blue * 0.986_039;
        let denominator = x + y + z;

        // TODO: if the z is truly the brightness we need to return it
        [x / denominator, y / denominator]
    }

    /// Convert from 'rgb' to the 'hsl' values that can be sent to the
    /// hue lights.
    pub fn rgb_to_hsl(rgb: Vec<u8>) -> (u16, u8, u8) {
        let standard: Vec<f32> = rgb
            .into_iter()
            .map(|val: u8| (val as f32) / 255.0)
            .collect();
        let (red, green, blue) = (standard[0], standard[1], standard[2]);
        let hsl: Hsl = Srgb::new(red, green, blue).into();
        let (h, s, l) = hsl.into_components();
        (
            (h.to_positive_degrees() / 360.0 * 65535.0) as u16,
            (s * 254.0) as u8,
            (l * 254.0) as u8,
        )
    }

    /// Convert hex color to `hsl`
    pub fn hex_to_hsl(s: &str) -> Result<(u16, u8, u8), std::num::ParseIntError> {
        let rgb = hex_to_rgb(s)?;
        Ok(rgb_to_hsl(rgb))
    }

    /// Convert hex color string to `rgb`
    pub fn hex_to_rgb(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect()
    }
}
