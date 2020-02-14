use clap::*;
use lighthouse::{colors, state, HueBridge, SendableState};

// TODO: if a light is provided by id then all the logic starts doing it on one light
// TODO: instead of printing out exit with error code
fn main() {
    let matches = clap_app!(lighthouse =>
                            (version: "0.0.1")
                            (author: "Art Eidukas <iwiivi@gmail.com>")
                            (about: "lighthouse - light automation from the comfort of your keyboard")
                            (@subcommand on =>
                             (about: "Turn all hue lights on")
                            )
                            (@subcommand off =>
                             (about: "Turn all hue lights off")
                            )
                            (@subcommand bri =>
                             (about: "Set brightness (turns on if off)")
                             (@arg bri: "Brightness value (0 - 254)")
                            )
                            (@subcommand state =>
                             (about: "Send state string to all hue lights")
                             (@arg filename: -f --file +takes_value "Filename if providing state from file. If provided ignores text string")
                             (@arg state: "Textual state to be sent")
                            )
                            (@subcommand info =>
                             (about: "Print out useful information about your system")
                            )
                            (@subcommand discover =>
                             (about: "Discover bridges on the network and print them")
                            )
                            (@subcommand loop =>
                             (about: "Set all lights to colorloop")
                            )
                            (@subcommand color =>
                             (about: "Color commands (WIP) the current API is unstable")
                             (@arg red: "rgb value of red")
                             (@arg green: "rgb value of green")
                             (@arg blue: "rgb value of blue")
                            )
    )
    .get_matches();

    if matches.subcommand_matches("discover").is_some() {
        println!(
            "Found the following bridges: {:?}",
            HueBridge::find_bridges()
        );
    } else {
        let h = HueBridge::connect();

        match matches.subcommand() {
            ("on", Some(_sub)) => {
                h.all(state!(on: true, bri: 254))
                    .expect("Error raised while turning all lights on");
            }
            ("off", Some(_sub)) => {
                h.all(state!(on: false))
                    .expect("Error raised while turning all lights off");
            }
            ("loop", Some(_sub)) => {
                h.all(state!(on: true, effect: "colorloop".into()))
                    .expect("Error raised while setting all lights to colorloop");
            }
            ("bri", Some(sub)) => match sub.value_of("bri") {
                Some(bri) => match bri.parse::<u8>() {
                    Ok(val) => {
                        h.all(state!(on: true, bri: val))
                            .expect("Error raised while adjusting brightness of all lights");
                    }
                    Err(e) => println!("Could not parse brightness value: {}", e),
                },
                None => println!("Missing brightness value"),
            },
            ("color", Some(sub)) => {
                match (
                    sub.value_of("red"),
                    sub.value_of("green"),
                    sub.value_of("blue"),
                ) {
                    (Some(red), Some(green), Some(blue)) => {
                        match (red.parse::<u8>(), green.parse::<u8>(), blue.parse::<u8>()) {
                            (Ok(red), Ok(green), Ok(blue)) => {
                                let xy = colors::rgb_to_xy(red, green, blue);
                                h.all(state!(on: true, colormode: "xy".into(), xy: xy))
                                    .expect("Error raised while setting color of all lights");
                            }
                            (_, _, _) => println!("Could not parse an rgb value"),
                        }
                    }
                    (_, _, _) => println!("Missing one rgb value"),
                }
            }
            ("state", Some(sub)) => {
                if let Some(filename) = sub.value_of("filename") {
                    if let Ok(file) = std::fs::File::open(filename) {
                        match serde_json::from_reader(std::io::BufReader::new(file)) {
                            Ok(state) => {
                                h.all(&state)
                                    .expect("Error raised while changing state of all lights");
                            }
                            Err(e) => println!("Could not parse state: {}", e),
                        }
                    }
                } else if let Some(state) = sub.value_of("state") {
                    match serde_json::from_str::<SendableState>(state) {
                        Ok(s) => {
                            h.all(&s)
                                .expect("Error raised while changing state of all lights");
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
