use crate::hue::*;
use reqwest::blocking::{Client, Response};
use serde_json::Value;

// TODO: Implement the send macro that
/// The Philips Hue light bridge.
pub struct HueBridge {
    pub address: String,
    pub client: Client,
    pub lights: LightCollection,
}

impl HueBridge {
    /// Discovers bridge IPs on the networks using SSDP
    pub fn find_bridges() -> Vec<String> {
        println!("Searching for bridges (5s)...");
        use ssdp::header::{HeaderMut, Man, MX, ST};
        use ssdp::message::{Multicast, SearchRequest};

        // create request with required headers for the sddp search
        let mut request = SearchRequest::new();
        request.set(Man);
        request.set(MX(5));
        request.set(ST::Target(ssdp::FieldMap::URN(
            "urn:schemas-upnp-org:device:Basic:1".into(),
        )));

        let mut bridges = Vec::new();
        for (_, src) in request.multicast().unwrap() {
            let ip = src.ip().to_string();
            if !bridges.contains(&ip) {
                bridges.push(ip)
            }
        }
        bridges
    }

    /// Waits for the user to press the button on the loop
    fn wait_for_button(ips: Vec<String>, client: &Client) -> (String, String) {
        let body = serde_json::json!({ "devicetype": "commandline::lighthouse" });
        let check = |i: &String| -> Value {
            client
                .post(&format!("http://{}/api", i))
                .json(&body)
                .send()
                .unwrap()
                .json()
                .unwrap()
        };

        let mut response: Value = Value::Null;
        let mut bridge_ip = String::new();
        loop {
            println!("Click the button on desired bridge.");
            std::thread::sleep(std::time::Duration::from_secs(3));
            for ip in &ips {
                response = check(&ip);
                if response[0]["error"]["type"] == 101 {
                    continue;
                } else {
                    bridge_ip.push_str(ip);
                    break;
                }
            }
            if response[0]["error"]["type"] != 101 {
                break;
            }
        }

        (bridge_ip, response[0]["success"]["username"].to_string())
    }

    fn authenticate(&mut self, configpath: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::{prelude::*, stdin};

        println!("Starting hue bridge registration procedure...");
        let bridges = Self::find_bridges(); // find any bridges present on network
        let mut ip = String::new(); // TODO: replace with proper IP struct

        let (ip, key) = if bridges.is_empty() {
            println!("Unfortunately, no bridges were found on the network.\nPlease supply an IP for a Hue bridge manually");
            stdin().read_line(&mut ip)?;
            ip = ip.trim().to_string();
            Self::wait_for_button(vec![ip], &self.client)
        } else {
            println!(
                "Bridge(s) found: {:?}\nWill try to connect to all of them sequentially...",
                &bridges
            );
            Self::wait_for_button(bridges, &self.client)
        };

        let mut file = std::fs::File::create(configpath)?;
        file.write_all(format!("HUE_BRIDGE_IP=\"{}\"\nHUE_BRIDGE_KEY={}", &ip, key).as_ref())?;
        println!("Config file successfully saved! (location: {})", configpath);

        if let Err(e) = dotenv::from_path(configpath) {
            println!(
                "Could not verify registration config file is valid: {} (check: {})",
                e, configpath
            );
        };

        self.address = format!("http://{}/api/{}/", ip, key.replace("\"", ""));
        Ok(())
    }

    /// This function orchestrates the authentication flow.
    /// It boils down to checking if the HUE Environment variables are
    /// loaded and if not kicking off the registration function.
    fn setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // load in a dotenv file
        let mut config = dirs::home_dir().unwrap();
        config.push(".lighthouse");

        // load in the config file
        if let Err(e) = dotenv::from_path(&config) {
            println!("NOTE: Could not load form config: {}", e);
            println!("Will try loading from environment variables anyway");
        };

        // get address from environment
        let ip = std::env::var("HUE_BRIDGE_IP");
        let key = std::env::var("HUE_BRIDGE_KEY");

        match (ip, key) {
            (Ok(ip), Ok(key)) => {
                self.address = format!("http://{}/api/{}/", ip, key);
            }
            (_, _) => {
                println!("Was not able to load from config or environment");
                if let Err(e) = self.authenticate(config.to_str().unwrap()) {
                    panic!("Registration failed: {}", e);
                };
            }
        }

        Ok(())
    }

    /// Establish a connection (Constructor method)
    pub fn connect() -> Self {
        let mut bridge = Self::default();

        if let Err(e) = bridge.setup() {
            println!("Could not complete setup: {}", e);
        };

        if let Err(e) = bridge.scan() {
            println!("Could not collect information about the system: {}", e);
        };

        bridge
    }

    /// Private function wrapping the request sending functionality
    pub fn request(
        &self,
        endpoint: &str,
        how: RequestType,
        params: Option<&SendableState>,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let target = format!("{}{}", self.address, endpoint);
        let response = match how {
            RequestType::Post => self.client.post(&target).json(&params).send()?,
            RequestType::Get => self.client.get(&target).send()?,
            RequestType::Put => self.client.put(&target).json(&params).send()?,
        };
        Ok(response)
    }

    /// Send state to each light associated with the ids passed into the function
    pub fn state_by_ids(
        &self,
        ids: &[u8],
        state: &SendableState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for id in ids {
            self.request(
                &format!("lights/{}/state", id),
                RequestType::Put,
                Some(state),
            )?;
        }
        Ok(())
    }

    // TODO: ADD send by name

    /// Send state to all lights that can be found on the
    /// bridge
    pub fn all(&self, state: &SendableState) -> Result<(), Box<dyn std::error::Error>> {
        self.state_by_ids(&self.lights.ids, state)
            .expect("Unable to send all lights by ids");
        Ok(())
    }

    /// Collects the lights that exist on the network and
    /// updates the struct with their:
    /// - ids
    /// - number of lights
    ///
    pub fn scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // collect the state of the system
        let lights: std::collections::BTreeMap<u8, Light> =
            self.request("lights", RequestType::Get, None)?.json()?;
        let ids: Vec<u8> = lights.keys().cloned().collect();
        let count = ids.len() as u8;

        self.lights = LightCollection { lights, ids, count };

        Ok(())
    }

    /// Print useful information about the state of your system
    pub fn system_info(&self) {
        match self.request("lights", RequestType::Get, None) {
            Ok(resp) => {
                let r: serde_json::Value = resp.json().unwrap();
                println!("{}", serde_json::to_string_pretty(&r).unwrap());
            }
            Err(e) => {
                println!("Could not send the get request: {}", e);
            }
        };
    }
}

impl Default for HueBridge {
    fn default() -> Self {
        Self {
            address: "".into(),
            client: Client::new(),
            lights: LightCollection::default(),
        }
    }
}

// TODO: Replace with normal reqwest enum
pub enum RequestType {
    Get,
    Put,
    Post,
}
