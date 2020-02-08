pub mod hue {
    // serde deserialisation
    use serde::*;
    use serde_json::Value;
    use std::collections::BTreeMap;

    /// A lights collection struct having useful information
    /// about the lights as well as their serialised form
    #[derive(Default)]
    pub struct LightCollection {
        pub lights: BTreeMap<u8, Light>,
        pub count: u8,
        pub ids: Vec<u8>,
    }

    /// Struct that can be sent to the Hue lights. It mirrors closely the
    /// `State`.
    ///
    /// ```
    /// let state_1: SendableState = serde_json::from_str(r#"{"on":true}"#);
    /// let state_2: SendableState = SendableState {on:true, ..SendableState::default()};
    /// let state_3: SendableState = state!(on: true, xy: [1.0, 0.123])
    /// ```
    #[derive(Serialize, Deserialize, Debug)]
    pub struct SendableState {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub on: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bri: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub hue: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sat: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub effect: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub xy: Option<[f32; 2]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub alert: Option<String>,
    }

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
            }
        }
    }

    /// Super useful macro to create `SendableState`
    /// ```
    /// // Usage example
    /// let sendable_state: SendableState = state!(on: true, xy: [1.0, 0.0])
    /// ```
    #[macro_export]
    macro_rules! state {
    ($($i:ident:$v:expr), *) => {
        lighthouse::lights::hue::SendableState {
            $($i: Some($v),) *
            ..lighthouse::lights::hue::SendableState::default()
        }
    };
}

    /// Light object representing the complete state of a light
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Light {
        pub state: State,
        swupdate: Value,
        pub r#type: String,
        pub name: String,
        pub modelid: String,
        pub manufacturername: String,
        pub productname: String,
        capabilities: Value,
        config: Value,
        pub uniqueid: String,
        swversion: String,
        swconfigid: String,
        pub productid: String,
    }

    impl std::fmt::Display for Light {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "
Name: {}
On: {:?}
Color: {:?}
",
                self.name, self.state.on, self.state.xy
            )
        }
    }
    /// This object contains the state part  of each light
    #[derive(Serialize, Deserialize, Debug)]
    pub struct State {
        pub on: bool,
        pub bri: u8,
        pub hue: u32,
        pub sat: u8,
        pub effect: String,
        pub xy: [f32; 2],
        pub ct: u32,
        pub alert: String,
        pub colormode: String,
        pub mode: String,
        pub reachable: bool,
    }

    // TODO: Input white normal state here as a default
    impl Default for State {
        fn default() -> Self {
            Self {
                on: false,
                bri: 254,
                hue: 1000,
                sat: 100,
                effect: "none".to_owned(),
                xy: [0.0, 0.0],
                ct: 300,
                alert: "select".to_owned(),
                colormode: "ct".to_owned(),
                mode: "homeautomation".to_owned(),
                reachable: true,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Capabilities {
        certified: bool,
        pub control: Control,
        pub ct: Value,
        streaming: Value,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Control {
        pub mindimlevel: u64,
        pub maxlumen: u64,
        pub colorgamuttype: String,
        pub colorgamut: [[f32; 2]; 3],
        pub ct: CTRange,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct CTRange {
        pub min: u16,
        pub max: u16,
    }
}
