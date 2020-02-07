use reqwest::blocking::{Client, Response};
use serde::*;

pub struct Hub {
    address: String,
    client: Client,
}

impl Hub {
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

    /// Establish a connection
    pub fn connect() -> Self {
        // get address
        let address = Self::get_address();

        // create the reqwest client
        let client = Client::new();

        Self { address, client }
    }

    /// Private function wrapping the request sending funtionality
    pub fn request(
        &self,
        endpoint: &str,
        how: RequestType,
        params: Option<impl Serialize>,
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
    pub fn state(
        &self,
        light: u8,
        state: impl Serialize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.request(
            &format!("lights/{}/state", light),
            RequestType::Put,
            Some(state),
        )?;
        Ok(())
    }
}

// TODO: Replace with normal reqwest enum
pub enum RequestType {
    Get,
    Put,
    Post,
}
