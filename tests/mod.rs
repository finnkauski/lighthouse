#[test]
fn test_generate_target() {
    use std::net::IpAddr;
    use url::Url;

    let addr: IpAddr = "192.168.0.1".parse().unwrap();
    assert_eq!(
        lighthouse::helpers::generate_target(addr, "tokengoeshere1234").unwrap(),
        Url::parse("http://192.168.0.1/api/tokengoeshere1234/").unwrap()
    )
}

#[test]
fn test_httpbin_get() {
    use lighthouse::helpers::network::{send_requests, AllowedMethod};
    let targets = vec![
        (
            url::Url::parse("http://httpbin.org/get").unwrap(),
            AllowedMethod::GET,
        ),
        (
            url::Url::parse("http://httpbin.org/post").unwrap(),
            AllowedMethod::POST,
        ),
        (
            url::Url::parse("http://httpbin.org/put").unwrap(),
            AllowedMethod::PUT,
        ),
    ];
    let states = vec![None, None, None];
    let client = reqwest::Client::new();
    let correct: bool = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(send_requests(targets, states, &client))
        .into_iter()
        .all(|r| r.unwrap().status() == 200);
    assert!(correct);
}

#[test]
fn test_empty_state_macro() {
    use lighthouse::{lights::*, *};
    let ref_state = state!();
    let nonref_state = state!();
    assert_eq!(ref_state, &SendableState::default());
    assert_eq!(nonref_state, &SendableState::default());
}

#[test]
fn test_state_macro() {
    use lighthouse::{lights::*, *};
    let ref_state = state!(on: true,
                                   bri: 230,
                                   hue: 100,
                                   sat: 20,
                                   effect: String::from("none"),
                                   xy: [1.0, 1.0],
                                   alert: String::from("none"),
                                   colormode: String::from("xy"),
                                   transitiontime: 2);
    let nonref_state = state!(nonref;
                                   on: true,
                                   bri: 230,
                                   hue: 100,
                                   sat: 20,
                                   effect: String::from("none"),
                                   xy: [1.0, 1.0],
                                   alert: String::from("none"),
                                   colormode: String::from("xy"),
                                   transitiontime: 2);
    let truth = SendableState {
        on: Some(true),
        bri: Some(230),
        hue: Some(100),
        sat: Some(20),
        effect: Some(String::from("none")),
        xy: Some([1.0, 1.0]),
        alert: Some(String::from("none")),
        colormode: Some(String::from("xy")),
        transitiontime: Some(2),
    };
    assert_eq!(ref_state, &truth);
    assert_eq!(nonref_state, truth);
}

#[test]
fn test_from_state() {
    use lighthouse::{lights::*, *};
    let mut s = State {
        on: true,
        bri: Some(100),
        hue: Some(240),
        sat: Some(20),
        effect: Some(String::from("none")),
        xy: Some([2.0, 2.0]),
        ct: Some(200),
        alert: String::from("select"),
        colormode: Some(String::from("somemode")),
        mode: String::from("mode"),
        reachable: true,
    };
    let state_default = state!(from: s.clone(););
    let state_changed = state!(from: s.clone(); on: false);

    assert_eq!(SendableState::from(s.clone()), state_default);

    s.on = false;
    assert_eq!(SendableState::from(s), state_changed);
}

#[test]
fn test_bridge_serialization() {
    use lighthouse::*;

    let filename = "test_bridge";
    // Create bridge from an IP and a Key.
    let b = bridge::Bridge::new("127.0.0.1".parse().unwrap(), "<SOME-KEY>".to_owned()).unwrap();

    b.to_file(filename).expect("Could not save bridge to file");

    let b2 = bridge::Bridge::from_file(filename).unwrap();

    assert!(b.target == b2.target);
}
