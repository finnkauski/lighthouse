// Example showing how to register a new bridge.
fn main() {
    use lighthouse::*;

    // Create bridge by registering with the Philips Hue Bridge
    let (b, token) = bridge::Bridge::try_register(true).unwrap();

    // Print out the whole bridge
    println!("Created bridge: {:#?}", b);
}
