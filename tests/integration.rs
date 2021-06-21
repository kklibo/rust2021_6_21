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

    assert_eq!(output.stdout, expected.as_bytes());

    Ok(())
}

