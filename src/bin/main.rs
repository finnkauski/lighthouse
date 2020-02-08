use lighthouse::controllers::HueBridge;

fn main() {
    let h = HueBridge::connect();
    h.doctor();
}
// TODO: Add interactive mode where the user talks to it like PG
