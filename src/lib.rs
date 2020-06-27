pub mod helpers {
    // imports
    use super::lights::SendableState;
    use std::net::IpAddr;
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

    type RequestBatch = Vec<(Url, AllowedMethod)>;
    type NewStates = Vec<Option<SendableState>>;
    type ResponseResult = Result<reqwest::Response, reqwest::Error>;

    /// Function wrapping the request sending functionality
    /// to a location.
    pub async fn future_request(
        target: Url,
        method: AllowedMethod,
        params: Option<SendableState>,
    ) -> ResponseResult {
        let client = reqwest::Client::new();
        match method {
            AllowedMethod::POST => client.post(target).json(&params).send().await,
            AllowedMethod::GET => client.get(target).send().await,
            AllowedMethod::PUT => client.put(target).json(&params).send().await,
        }
    }

    /// Function that can generate a asyncronous object
    /// that will go off
    #[tokio::main]
    pub async fn requests(request_batch: RequestBatch, states: NewStates) -> Vec<ResponseResult> {
        futures::future::join_all(
            request_batch
                .into_iter()
                .zip(states.into_iter())
                .map(|((url, method), state)| future_request(url, method, state)),
        )
        .await
    }
}

pub mod bridge {
    // imports
    use super::helpers::*;
    use std::net::IpAddr;
    use url::Url;

    type BridgeError = String;

    #[derive(Debug)]
    pub struct Bridge {
        pub target: Url,
    }

    impl Bridge {
        /// Constructor for a bridge from and IP and a token
        pub fn new(ip: IpAddr, token: &str) -> Result<Self, BridgeError> {
            let target = generate_target(ip, token)?;
            Ok(Bridge { target })
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

    /// Struct that can be sent to the Hue lights. It mirrors closely the
    /// `State`.
    ///
    /// ```
    /// use lighthouse::*;
    /// let state_1: SendableState = serde_json::from_str(r#"{"on":true}"#).unwrap();
    /// let state_2: SendableState = SendableState {on:Some(true), ..SendableState::default()};
    /// let state_3: SendableState = state!(on: true, xy: [1.0, 0.123]);
    /// let state_4: &SendableState = state!(ref, on: true, xy: [1.0, 0.123]);
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

    /// Super useful macro to create `SendableState`
    /// ```
    /// use lighthouse::*;
    /// // Usage examples
    /// // Returns a reference, most useful as a default
    /// let sendable_state: &SendableState = state!(on: true, xy: [1.0, 0.0]);
    ///
    /// // Returns a value, still useful.
    /// let sendable_state: SendableState = state!(nonref, on: true, xy: [1.0, 0.0]);
    /// ```
    #[macro_export]
    macro_rules! state {
    (ref, $($i:ident:$v:expr), *) => {
        &SendableState {
            $($i: Some($v),) *
            ..SendableState::default()
        }
    };

    ($($i:ident:$v:expr), *) => {
        SendableState {
            $($i: Some($v),) *
            ..SendableState::default()
        }
    };

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
                Url::parse("https://192.168.0.1/api/tokengoeshere1234/").unwrap()
            )
        }

        #[test]
        fn test_httpbin_get() {
            use crate::helpers::{request, AllowedMethod};
            assert_eq!(
                request(
                    url::Url::parse("https://httpbin.org/get").unwrap(),
                    AllowedMethod::GET,
                    None
                )
                .status(),
                200
            );
            assert_eq!(
                request(
                    url::Url::parse("https://httpbin.org/put").unwrap(),
                    AllowedMethod::PUT,
                    None
                )
                .status(),
                200
            );
            assert_eq!(
                request(
                    url::Url::parse("https://httpbin.org/post").unwrap(),
                    AllowedMethod::POST,
                    None
                )
                .status(),
                200
            );
        }
    }
}
