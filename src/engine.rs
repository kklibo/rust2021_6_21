//! The transaction processing engine

use std::error::Error;

use crate::input::InputRecord;


/// A client ID
#[derive(PartialEq,Debug)]
struct ClientId(u16);

/// A globally-unique transaction ID
#[derive(PartialEq,Debug)]
struct TxId(u32);

/// A deposit or withdrawal amount; expected precision is 4 places past the decimal
#[derive(PartialEq,Debug)]
struct Amount(f32);

/// A transaction that applies to a client account
#[derive(PartialEq,Debug)]
enum Transaction {
    Deposit(TxId, Amount),
    Withdrawal(TxId, Amount),
}

/// Parses an InputRecord into a client ID + Transaction pair
fn parse_record(record: &InputRecord) -> Result<(ClientId, Transaction), Box<dyn Error>> {

    match record {

        InputRecord{r#type, client,tx, amount: Some(amount)} => {

            match r#type.as_str() {
                "deposit"    => Ok((ClientId(*client), Transaction::Deposit(TxId(*tx), Amount(*amount)))),
                "withdrawal" => Ok((ClientId(*client), Transaction::Withdrawal(TxId(*tx), Amount(*amount)))),
                _ => Err("invalid input record".into())

            }
        }

        _ => Err("invalid input record".into())
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_record_test() {

        //success: deposit
        {
            let record = InputRecord{r#type: "deposit".to_string(), client: 1, tx: 2, amount: Some(3.0)};
            let result = parse_record(&record).unwrap();

            assert_eq!(result, (ClientId(1), Transaction::Deposit(TxId(2), Amount(3.0))));
        }

        //success: withdrawal
        {
            let record = InputRecord{r#type: "withdrawal".to_string(), client: 1, tx: 2, amount: Some(3.0)};
            let result = parse_record(&record).unwrap();

            assert_eq!(result, (ClientId(1), Transaction::Withdrawal(TxId(2), Amount(3.0))));
        }

        //failure: nonexistent transaction type
        {
            let record = InputRecord{r#type: "no_such_tx_type".to_string(), client: 1, tx: 2, amount: None};
            let result = parse_record(&record);

            assert!(matches!(result, Err(_)));
        }

        //failure: deposit is missing its amount
        {
            let record = InputRecord{r#type: "deposit".to_string(), client: 1, tx: 2, amount: None};
            let result = parse_record(&record);

            assert!(matches!(result, Err(_)));
        }
    }
}