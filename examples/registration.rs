// Example showing how to register a new bridge.
fn main() {
    use lighthouse::*;

    // Create bridge by registering with the Philips Hue Bridge
    let mut b = bridge::Bridge::try_register(true);

    // Print out the whole bridge
    println!("Created bridge: {:#?}", b);
}
