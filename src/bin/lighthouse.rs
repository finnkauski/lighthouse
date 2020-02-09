use clap::*;
use lighthouse::{state, HueBridge, SendableState};

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
    )
    .get_matches();

    if matches.subcommand_matches("discover").is_none() {
        let h = HueBridge::connect();
        if matches.subcommand_matches("on").is_some() {
            h.all(state!(on: true, bri: 254));
        } else if matches.subcommand_matches("off").is_some() {
            h.all(state!(on: false));
        } else if let Some(sub) = matches.subcommand_matches("state") {
            if let Some(filename) = sub.value_of("filename") {
                if let Ok(file) = std::fs::File::open(filename) {
                    match serde_json::from_reader(std::io::BufReader::new(file)) {
                        Ok(state) => {
                            h.all(&state);
                        }
                        Err(e) => println!("Could not parse state: {}", e),
                    }
                }
            } else if let Some(state) = sub.value_of("state") {
                match serde_json::from_str::<SendableState>(state) {
                    Ok(s) => {
                        h.all(&s);
                    }
                    Err(e) => println!("Unable to parse text state: {}", e),
                }
            }
        } else if matches.subcommand_matches("info").is_some() {
            h.system_info();
        }
    } else {
        println!(
            "Found the following bridges: {:?}",
            HueBridge::find_bridges()
        );
    };
}
// TODO: Add interactive mode where the user talks to it like PG
