use crate::Error;
use clap::ArgMatches;

use crate::config::Config;
use crate::helpers;

pub fn cmd(matches: &ArgMatches) -> Result<(), Error> {
    let mut config = Config::read();

    let ch = match helpers::parse_channel(matches)? {
        None => {
            // if no channel is provided, print the current configuration
            print!("{}", config);
            return Ok(());
        }
        Some(ch) => ch,
    };

    config.channels[ch].enabled = !matches.is_present("disable");

    config.channels[ch].loopback = matches.is_present("loopback");

    config.channels[ch].monitor = matches.is_present("monitor");

    config.channels[ch].fd = matches.is_present("fd");

    if matches.is_present("bitrate") {
        let bitrate = match matches.value_of("bitrate").unwrap().parse::<u32>() {
            Err(_) => {
                return Err(Error::InvalidArgument(String::from(
                    "invalid bitrate value",
                )))
            }
            Ok(b) => b,
        };
        config.channels[ch].bitrate = bitrate;
    }

    if matches.is_present("data_bitrate") {
        let data_bitrate = match matches.value_of("data_bitrate").unwrap().parse::<u32>() {
            Err(_) => {
                return Err(Error::InvalidArgument(String::from(
                    "invalid data_bitrate value",
                )))
            }
            Ok(b) => b,
        };
        config.channels[ch].data_bitrate = data_bitrate;
    }

    config.write().unwrap();

    print!("{}", config);
    Ok(())
}
