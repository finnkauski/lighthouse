//! # lighthouse
//!
//! The `lighthouse` crate provides a wrapper for the Philips Hue REST API provided on a local
//! network by the Philips Hue Bridge.
//!
//! ## Constucting the Bridge client and finding your lights
//!
//! ```no_run
//! use std::net::{IpAddr, Ipv4Addr};
//! use lighthouse::bridge::Bridge;
//! let ip_addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10));
//! let bridge_token = String::from("my-example-token");
//! let mut bridge = Bridge::new(ip_addr, bridge_token).unwrap();
//! let lights = bridge.scan();
//! ```
//!
//! ## Controlling individual lights
//!
//! ```no_run
//! use lighthouse::state;
//! use lighthouse::bridge::Bridge;
//! let mut bridge = Bridge::new("192.168.1.10".parse().unwrap(), "token".to_string()).unwrap();
//! bridge.state_to(1, state!(on: true, bri: 128));
//! ```

// TODO: Implement a Bridge Builder and move the building functions out of the actual bridge
// TODO: Add validation check for when making a bridge - ping some API endpoint to collect data. Good way to get more info as well about the bridge

pub mod bridge;
#[cfg(feature = "color")]
pub mod color;
pub mod helpers;
pub mod lights;
