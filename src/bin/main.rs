use lighthouse::{controllers::Hub, state};

fn main() {
    let h = Hub::connect();
    h.state(3, state!(on: true));
}
