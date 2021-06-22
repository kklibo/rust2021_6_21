//! Integration tests


/// Using the assert_cmd crate to conveniently run this crate's binary
use assert_cmd::prelude::*;

use std::process::Command;
use std::error::Error;


const BIN_NAME: &str = env!("CARGO_PKG_NAME");


///test the example in the spec
#[test]
fn basic_test() -> Result<(), Box<dyn Error>> {

    let output = Command::cargo_bin(BIN_NAME)?
                .arg("tests/basic_test.csv")
                .output()?;

    let expected =
"client, available, held, total, locked
1,1.5000,0.0000,1.5000,false
2,2.0000,0.0000,2.0000,false
";

    let output_str = std::str::from_utf8(&output.stdout)?;
    assert_eq!(output_str, expected);

    Ok(())
}

///test a general set of program behaviors
#[test]
fn general_test() -> Result<(), Box<dyn Error>> {

    let output = Command::cargo_bin(BIN_NAME)?
                .arg("tests/general_test.csv")
                .output()?;

    let expected =
"client, available, held, total, locked
1,0.0000,0.0000,0.0000,false
2,-2.0000,0.0000,-2.0000,true
3,1.0000,2.0000,3.0000,false
";

    let output_str = std::str::from_utf8(&output.stdout)?;
    assert_eq!(output_str, expected);

    Ok(())
}