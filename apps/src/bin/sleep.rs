#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

#[macro_use]
extern crate user_lib;

use user_lib::sync::sleep;

const MS_PER_SECOND: usize = 1000;
const SECONDS_PER_MINUTE: usize = 60;
const SECONDS_PER_HOUR: usize = 3600;
const SECONDS_PER_DAY: usize = 86400;

#[no_mangle]
extern "Rust" fn main(_argc: usize, argv: &[&str]) -> i32 {
    let mut total_sleep_ms = 0;

    for &arg in argv.iter().skip(1) {
        if arg.is_empty() {
            continue;
        }
        total_sleep_ms += match parse_time_argument(arg) {
            Ok(ms) => ms,
            Err(e) => {
                println!("Error parsing time argument '{}': {}", arg, e);
                return 1;
            }
        };
    }

    sleep(total_sleep_ms);
    0
}

fn parse_time_argument(arg: &str) -> Result<usize, &'static str> {
    if let Ok(time) = arg.parse::<usize>() {
        Ok(time * MS_PER_SECOND)
    } else {
        let (num_part, unit) = arg.split_at(arg.len() - 1);
        let number = num_part.parse::<usize>().map_err(|_| "Invalid number")?;
        let ms = match unit {
            "s" => number * MS_PER_SECOND,
            "m" => number * SECONDS_PER_MINUTE * MS_PER_SECOND,
            "h" => number * SECONDS_PER_HOUR * MS_PER_SECOND,
            "d" => number * SECONDS_PER_DAY * MS_PER_SECOND,
            _ => return Err("Unsupported time unit"),
        };
        Ok(ms)
    }
}
