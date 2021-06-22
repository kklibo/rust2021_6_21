//! Processes input CSVs

use std::error::Error;
use csv::{ReaderBuilder,Trim};
use serde::Deserialize;


#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize)]
///A typed representation of a single input line
pub struct InputRecord {
    pub r#type: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f32>,
}

///Parses a CSV string into InputRecords
pub fn parse_csv(input_csv: String) -> Result<Vec<InputRecord>, Box<dyn Error>> {

    let mut reader = ReaderBuilder::new()
            .flexible(true)
            .trim(Trim::All)
            .from_reader(input_csv.as_bytes());

    let mut records = Vec::<InputRecord>::new();

    for record in reader.deserialize() {

        records.push(record?);
    }

    Ok(records)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_csv_test() {

        //empty
        assert_eq!(parse_csv("".to_string()).unwrap(), vec![]);

        //just the header
        assert_eq!(parse_csv("type,client,tx,amount".to_string()).unwrap(), vec![]);


        //one record
        {
            let result = parse_csv(
"type, client,  tx, amount
deposit,1,2,3.000"
                    .to_string()).unwrap();

            let expected = vec! [
                InputRecord {r#type: "deposit".to_string(), client: 1, tx: 2, amount: Some(3.0)},
            ];

            assert_eq!(result, expected);
        }

        //leading/interspersed whitespace
        {
            let result = parse_csv(
"type, client,  tx, amount
deposit ,1,2,  3.0
   withdrawal,4,5,      6.0
chargeback,      7,8"
                    .to_string()).unwrap();

            let expected = vec! [
                InputRecord {r#type: "deposit".to_string(), client: 1, tx: 2, amount: Some(3.0)},
                InputRecord {r#type: "withdrawal".to_string(), client: 4, tx: 5, amount: Some(6.0)},
                InputRecord {r#type: "chargeback".to_string(), client: 7, tx: 8, amount: None},
            ];

            assert_eq!(result, expected);
        }

        //failure: record line is too short
        {
            let result = parse_csv(
"type, client,  tx, amount
deposit,1"
                    .to_string());

            assert!(matches!(result, Err(_)));
        }


        //failure: record element is the wrong type
        {
            let result = parse_csv(
"type, client,  tx, amount
deposit,not_an_integer,2,3.000"
                    .to_string());

            assert!(matches!(result, Err(_)));
        }
    }
}