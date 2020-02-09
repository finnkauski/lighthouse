use lighthouse::{state, HueBridge};

fn main() {
    // connect to HueBridge using its registration procedure
    // if no credentials are found otherwise using detected
    // credentials
    let h = HueBridge::connect();

    // if an id is known send it to the light
    // default light here is 1.
    //
    // state macro creates a struct that serialises
    // to a valid state being sent to the light
    h.state_by_id(1, state!(on: true, bri: 254));

    // the following line sends the state to all lights
    // that can be detected on the system
    h.all(state!(on: true, bri: 254));
}
