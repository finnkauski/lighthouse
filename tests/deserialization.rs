use lighthouse::lights::*;

#[test]
fn lightbulb_from_json() {
    if let Ok(Light::LightBulb { .. }) =
        serde_json::from_str(include_str!("json_examples/light.json"))
    {
    } else {
        panic!("Could not deserialize the lights into the correct struct type: LightBulb")
    }
}

#[test]
fn lightstrip_from_json() {
    if let Ok(Light::LightStrip { .. }) =
        serde_json::from_str(include_str!("json_examples/lightstrip.json"))
    {
    } else {
        panic!("Could not deserialize the lights into the correct struct type: LightBulb")
    }
}
