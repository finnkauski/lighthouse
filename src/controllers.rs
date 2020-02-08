#[macro_use]
use crate::{lights::hue::*};
use reqwest::blocking::{Client, Response};

/// The Philips Hue light hub.
///
///
pub struct HueBridge {
    address: String,
    client: Client,
    lights: LightCollection,
}

impl HueBridge {
    /// Load configs from the environment
    fn get_address() -> String {
        // load in a dotenv file
        let mut config = dirs::home_dir().unwrap();
        config.push(".lighthouse");
        dotenv::from_path(config);

        // get address from environment
        let ip = std::env::var("HUE_BRIDGE_IP").unwrap();
        let key = std::env::var("HUE_BRIDGE_KEY").unwrap();

        format!("http://{}/api/{}/", ip, key)
    }

    /// Establish a connection (Constructor method)
    pub fn connect() -> Self {
        // get address
        let address = Self::get_address();

        // create the reqwest client
        let client = Client::new();

        let mut bridge = Self {
            address,
            client,
            lights: LightCollection::default(),
        };

        if let Err(e) = bridge.scan() {
            println!("Could not collect information about the system: {}", e);
        };

        bridge
    }

    /// Private function wrapping the request sending funtionality
    pub fn request(
        &self,
        endpoint: &str,
        how: RequestType,
        params: Option<SendableState>,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let target = format!("{}{}", self.address, endpoint);
        let response = match how {
            RequestType::Post => self.client.post(&target).json(&params).send()?,
            RequestType::Get => self.client.get(&target).send()?,
            RequestType::Put => self.client.put(&target).json(&params).send()?,
        };
        Ok(response)
    }

    /// Send state to the light endpoint
    pub fn state(&self, light: u8, state: SendableState) -> Result<(), Box<dyn std::error::Error>> {
        self.request(
            &format!("lights/{}/state", light),
            RequestType::Put,
            Some(state),
        )?;
        Ok(())
    }

    /// Collects the lights that exist on the network and
    /// updates the struct with their:
    /// - ids
    /// - number of lights
    ///
    fn scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // collect the state of the system
        let lights: std::collections::BTreeMap<u8, Light> =
            self.request("lights", RequestType::Get, None)?.json()?;
        let ids: Vec<u8> = lights.keys().cloned().collect();
        let count = ids.len() as u8;

        self.lights = LightCollection { lights, ids, count };

        Ok(())
    }

    /// Print outs diagnostic information for debuging purposes
    /// Currently printed:
    /// - response from the `lights` endpoint
    pub fn doctor(&self) {
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

// TODO: Replace with normal reqwest enum
pub enum RequestType {
    Get,
    Put,
    Post,
}
