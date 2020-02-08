use crate::hue::*;
use reqwest::blocking::{Client, Response};

// TODO: Implement the send macro that
/// The Philips Hue light bridge.
pub struct HueBridge {
    address: String,
    client: Client,
    lights: LightCollection,
}

impl HueBridge {
    /// Load configs from the environment
    fn authenticate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // load in a dotenv file
        let mut config = dirs::home_dir().unwrap();
        config.push(".lighthouse");
        dotenv::from_path(config);

        // get address from environment
        let ip = std::env::var("HUE_BRIDGE_IP").unwrap();
        let key = std::env::var("HUE_BRIDGE_KEY").unwrap();

        self.address = format!("http://{}/api/{}/", ip, key);

        Ok(())
    }

    /// Establish a connection (Constructor method)
    pub fn connect() -> Self {
        let mut bridge = Self::default();

        if let Err(e) = bridge.authenticate() {
            println!("Could not authenticate: {}", e);
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

    /// Send state to the light endpoint
    pub fn state_by_id(
        &self,
        light: u8,
        state: &SendableState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.request(
            &format!("lights/{}/state", light),
            RequestType::Put,
            Some(state),
        )?;
        Ok(())
    }

    // TODO: ADD send by name

    /// Send state to all lights that can be found on the
    /// bridge
    pub fn all(&self, state: &SendableState) -> Result<(), Box<dyn std::error::Error>> {
        for id in &self.lights.ids {
            self.request(
                &format!("lights/{}/state", id),
                RequestType::Put,
                Some(state),
            )?;
        }
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
