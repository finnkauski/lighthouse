use clap::*;
use lighthouse::{colors, state, HueBridge, SendableState};

// TODO: if a light is provided by id then all the logic starts doing it on one light
// TODO: instead of printing out exit with error code
fn main() {
    let matches = create_app().get_matches();

    if matches.subcommand_matches("discover").is_some() {
        println!(
            "Found the following bridges: {:?}",
            HueBridge::find_bridges()
        );
    } else {
        let h = HueBridge::connect();
        let ids: Vec<u8> = if let Some(matches) = matches.values_of("ids") {
            // TODO: use a validator
            matches.map(|s: &str| s.parse().unwrap()).collect()
        } else {
            Vec::new()
        };
        let run = |state: &SendableState, err: &str| {
            if ids.is_empty() {
                h.all(state).expect(err);
            } else {
                h.state_by_ids(&ids, state).expect(err);
            }
        };
        match matches.subcommand() {
            ("on", Some(_sub)) => {
                let state = state!(on: true, bri: 254);
                let err = "Error raised while turning all lights on";
                run(state, err);
            }
            ("off", Some(_sub)) => {
                let state = state!(on: false);
                let err = "Error raised while turning all lights off";
                run(state, err);
            }
            ("loop", Some(_sub)) => {
                let state = state!(on: true, effect: "colorloop".into());
                let err = "Error raised while turning lights to colorloop";
                run(state, err);
            }
            ("bri", Some(sub)) => match sub.value_of("bri") {
                Some(bri) => match bri.parse::<u8>() {
                    Ok(val) => {
                        let state = state!(on: true, bri: val);
                        run(state, "Error raised while adjusting brightness of lights");
                    }
                    Err(e) => println!("Could not parse brightness value: {}", e),
                },
                None => println!("Missing brightness value"),
            },
            ("color", Some(sub)) => match sub.value_of("hexcode") {
                Some(hex) => match colors::hex_to_hsl(hex.into()) {
                    Ok((h, s, l)) => {
                        run(
                            state!(on: true, colormode: "hs".into(), hue: h, sat: s, bri: l),
                            "Error raised while setting color of all lights",
                        );
                    }
                    _ => println!("Could not parse hex value: {}", hex),
                },
                _ => println!("Hexcode not provided"),
            },
            ("state", Some(sub)) => {
                if let Some(filename) = sub.value_of("filename") {
                    if let Ok(file) = std::fs::File::open(filename) {
                        match serde_json::from_reader(std::io::BufReader::new(file)) {
                            Ok(state) => {
                                run(&state, "Error raised while changing state of all lights");
                            }
                            Err(e) => println!("Could not parse state: {}", e),
                        }
                    }
                } else if let Some(state) = sub.value_of("state") {
                    match serde_json::from_str::<SendableState>(state) {
                        Ok(s) => {
                            run(&s, "Error raised while changing state of all lights");
                        }
                        Err(e) => println!("Unable to parse text state: {}", e),
                    }
                }
            }
            ("info", Some(_sub)) => {
                h.system_info();
            }
            _ => println!("No command passed. type: `lh --help`"),
        }
    }
}
// TODO: Add interactive mode where the user talks to it like PG
// TODO: order commands in a nice way
// TODO: RGB based color
// TODO: refactor so you pass stuff with flags
// TODO: longer delay for discover
fn create_app() -> clap::App<'static, 'static> {
    App::new("lightouse")
        .version("0.1.0")
        .author("Art Eidukas <iwiivi@gmail.com>")
        .about("lighthouse - light automation from the comfort of your keyboard")
        .arg(
            Arg::with_name("ids")
                .short("i")
                .long("ids")
                .value_name("IDs")
                .help("Comma delimited IDs of lights that will get affected by the command")
                .takes_value(true)
                .use_delimiter(true)
                .global(true)
        )
        .subcommands(vec![
            SubCommand::with_name("on").about("Turn hue lights on"),
            SubCommand::with_name("off").about("Turn hue lights off"),
            SubCommand::with_name("bri")
                .about("Set brightness (turns lights on)")
                .arg(
                    Arg::with_name("bri")
                        .value_name("BRIGHTNESS")
                        .takes_value(true),
                ),
            SubCommand::with_name("state")
                .about("Manually send state to hue lights")
                .arg(
                    Arg::with_name("filename")
                        .short("f")
                        .long("file")
                        .value_name("FILE")
                        .takes_value(true)
                        .help("Filename if providing state from file. If provided ignores text string")
                )
                .arg(
                    Arg::with_name("state")
                        .value_name("STATE")
                        .required(true)
                        .takes_value(true)
                        .help("Textual state to be sent")
                        .required_if("filename", "")
                ),
            SubCommand::with_name("info").about("Print out useful information about your system"),
            SubCommand::with_name("discover").about("Discover bridges on the network and print them"),
            SubCommand::with_name("loop").about("Set lights to colorloop"),
            SubCommand::with_name("color").about("Send colors to lights")
            .arg(
                    Arg::with_name("hexcode")
                        .value_name("HEXCODE")
                        .required(true)
                        .takes_value(true)
                        .help("Hex code for desired color (no hash)")
            ),

        ])
}
