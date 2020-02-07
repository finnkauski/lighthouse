use lighthouse::controllers::{Hub, RequestType};

fn main() {
    let h = Hub::connect();
    dbg!(h
        .request("lights", RequestType::Get, Some(serde_json::Value::Null))
        .unwrap()
        .text());
}
