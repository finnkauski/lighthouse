/// # Lights module
///
/// This module contains the core representations for the lights and responses
/// from the API.
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
            Self::LightBulb { state, name, .. } => write!(f, "{{ on: {} }} : {}", state.on, name),
            Self::LightStrip { state, name, .. } => write!(f, "{{ on: {} }} : {}", state.on, name),
        }
    }
}

/// Light enum representing the complete state of possible lights
#[derive(Serialize, Deserialize, Debug, Clone)]
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
