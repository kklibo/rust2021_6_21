use std::error::Error;
use std::fs::read_to_string;

use rust2021_6_21::input::parse_csv;
use rust2021_6_21::engine::run;


/// A very lightweight main function:
/// The spec doesn't require specific error behavior,
/// so errors are just directly returned as soon as they're encountered.
fn main() -> Result<(), Box<dyn Error>>{

    let args: Vec<String> = std::env::args().collect();

    let csv = read_to_string(args.get(1).ok_or("Specify input path")?)?;

    let records = parse_csv(csv)?;

    let account_states = run(&records);


    println!("client, available, held, total, locked");

    for account_state in account_states {
        println!("{}", account_state);
    }

    Ok(())
}
