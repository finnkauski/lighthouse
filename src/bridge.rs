/// # Bridge module
///
/// This module contains the Bridge and related functionality
// imports
use super::{
    helpers::{network::*, *},
    lights::*,
};
use std::cell::RefCell;
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
    runtime: RefCell<Runtime>,
    lights: RefCell<BTreeMap<u8, Light>>,
}

impl Bridge {
    /// Constructor for a bridge from and IP and a token
    pub fn new(ip: IpAddr, token: String) -> Result<Self, ()> {
        let target = generate_target(ip, &token)?;
        let client = reqwest::Client::new();
        let runtime = RefCell::new(
            tokio::runtime::Builder::new()
                .basic_scheduler()
                .enable_all()
                .build()
                .expect("Could not create tokio runtime during the creation of the Bridge"),
        );
        let lights = RefCell::new(BTreeMap::new());
        Ok(Bridge {
            target,
            ip,
            token,
            client,
            runtime,
            lights,
        })
    }

    /// Scan the existing lights on the network. Returns the light id
    /// mapped to the light object.
    pub fn scan(&self) -> BTreeMap<u8, Light> {
        let endpoint = self.get_endpoint("./lights", AllowedMethod::GET);
        let fut = send_request(endpoint, None, &self.client);
        let lights: BTreeMap<u8, Light> = self
            .runtime
            .borrow_mut()
            .block_on(async { fut.await?.json().await })
            .expect("Could not completely decode/send request");
        lights
    }

    /// Updates the lights in the system by scanning them
    fn update_lights(&self) {
        self.lights.replace(self.scan());
    }

    /// Get the lights from the bridge struct. It performs a rescan and
    /// updates a private lights field in the Bridge.
    pub fn get_lights(&self) -> BTreeMap<u8, Light> {
        self.update_lights();
        self.lights.borrow().clone()
    }

    /// sends a state to a given light by its ID on the system.
    ///
    /// This is useful when you want to send a given state to one light
    /// on the network.
    pub fn state_to(&mut self, id: u8, new_state: &SendableState) -> reqwest::Response {
        let endpoint = self.get_endpoint(&format!("./lights/{}/state", id)[..], AllowedMethod::PUT);
        self.runtime
            .borrow_mut()
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
            .map(|id| self.get_endpoint(&format!("./lights/{}/state", id)[..], AllowedMethod::PUT))
            .collect();
        let states = new_states.into_iter().map(Some);

        self.runtime
            .borrow_mut()
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
    ///
    /// The interactive parameter, if true, will enable printing out of instructions to
    /// stdout.
    ///
    /// This returns a [Bridge](struct.Bridge.html) and a token. The reason for a
    /// returned token is that a user might want to store a token, however the struct
    /// field is private by default on the bridge, so we expose the token upon registration
    /// for the user to store it as they might see fit.
    pub fn try_register(interactive: bool) -> Result<(Self, String), String> {
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
                println!("Will try to register. Please press the connection button on your bridge");
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

        Ok((
            Bridge {
                target,
                ip: *bridge_ip,
                token: token.clone(),
                client,
                runtime: RefCell::new(runtime),
                lights: RefCell::new(Default::default()),
            },
            token,
        ))
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
            .borrow_mut()
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
