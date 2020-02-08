use clap::*;
use lighthouse::{state, HueBridge, SendableState};

fn main() {
    let h = HueBridge::connect();
    let matches = clap_app!(lighthouse =>
                            (version: "0.0.1")
                            (author: "Art Eidukas <iwiivi@gmail.com>")
                            (about: "lighthouse - light automation from the comfort of your keyboard")
                            (@subcommand on =>
                             (about: "turn all hue lights on")
                            )
                            (@subcommand off =>
                             (about: "turn all hue lights off")
                            )
                            (@subcommand state =>
                             (about: "send state string to all hue lights")
                             (@arg state: +required)
                            )

    )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("on") {
        h.all(state!(on: true, bri: 254));
    } else if let Some(_) = matches.subcommand_matches("off") {
        h.all(state!(on: false));
    } else if let Some(sub) = matches.subcommand_matches("state") {
        if let Some(state) = sub.value_of("state") {
            match serde_json::from_str::<SendableState>(state) {
                Ok(s) => {
                    h.all(&s);
                }
                Err(e) => println!("Unable to parse text state: {}", e),
            }
        }
    }
}
// TODO: Add interactive mode where the user talks to it like PG
