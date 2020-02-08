use lighthouse::{state, HueBridge};

fn main() {
    let h = HueBridge::connect();
    h.all(state!(on: true));
    std::thread::sleep(std::time::Duration::from_secs(1));
    h.all(state!(on: false));
}
// TODO: Add interactive mode where the user talks to it like PG
