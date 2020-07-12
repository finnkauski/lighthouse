// Save the bridge to file and load it back in;
fn main() {
    use lighthouse::*;

    let filename = "test_bridge";
    // Create bridge from an IP and a Key.
    let mut b = bridge::Bridge::new("127.0.0.1".parse().unwrap(), "<SOME-KEY>".to_owned()).unwrap();

    b.to_file(filename);
    bridge::Bridge::from_file(filename);
}
