//! The transaction processing engine

use std::collections::HashMap;
use std::error::Error;

use crate::input::InputRecord;
use crate::account_state::AccountState;


/// A client ID
#[derive(Copy,Clone,Eq,PartialEq,Hash,Debug)]
pub struct ClientId(pub u16);

/// A globally-unique transaction ID
#[derive(Copy,Clone,PartialEq,Debug)]
struct TxId(u32);

/// A deposit or withdrawal amount; expected precision is 4 places past the decimal
#[derive(Copy,Clone,PartialEq,Debug)]
pub struct Amount(pub f32);

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

/// Processes an account's transaction history and returns its current state.
/// Note: `client_id` is only used to create the AccountState:
/// all `transactions` will be processed.
fn process_account_transactions(client_id: ClientId, transactions: &Vec<Transaction>) -> AccountState {

    let mut account_state = AccountState {
        client_id,
        available: Amount(0.0),
        held: Amount(0.0),
        locked: false
    };

    for transaction in transactions {

        match transaction {

            Transaction::Deposit(_tx_id, amount) => {

                //deposits always succeed
                account_state.available.0 += amount.0;
            },

            Transaction::Withdrawal(_tx_id, amount) => {

                //withdrawals only happen if enough funds are available
                if account_state.available.0 >= amount.0 {
                    account_state.available.0 -= amount.0;
                }
            },
        }
    }

    account_state
}

///Processes a history of transactions:
/// calculates and returns the resulting state of each client account
pub fn run(records: &Vec<InputRecord>) -> Vec<AccountState> {

    //maps a client ID to an ordered sequence of transactions on its account
    let mut account_histories = HashMap::<ClientId, Vec<Transaction>>::new();

    for record in records {

        match parse_record(record) {
            Ok((client_id, transaction)) => {

                //add this transaction to the client ID's transaction sequence
                account_histories.entry(client_id).or_default().push(transaction);
            },

            //The spec doesn't specify an error-reporting channel. What could be done here?
            // For now, just ignore invalid records.
            Err(_) => {},
        }
    }

    //process the histories of the client accounts:
    // generate an AccountState for each
    account_histories.iter().map(|(&client_id, transactions)| {
        process_account_transactions(client_id, transactions)
    }).collect()
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

    #[test]
    fn process_account_transactions_test() {

        let client_id = ClientId(1);

        //no transactions
        {
            let transactions = vec![];

            let expected = AccountState {
                client_id,
                available: Amount(0.0),
                held: Amount(0.0),
                locked: false
            };

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }

        //deposits + withdrawals (all successful)
        {
            let transactions = vec![
                Transaction::Deposit(TxId(1), Amount(10.0)),
                Transaction::Deposit(TxId(2), Amount(1.0)),
                Transaction::Withdrawal(TxId(3), Amount(2.0)),
                Transaction::Deposit(TxId(4), Amount(1.0)),
            ];

            let expected = AccountState {
                client_id,
                available: Amount(10.0),
                held: Amount(0.0),
                locked: false
            };

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }

        //overdrawing withdrawal rejected
        {
            let transactions = vec![
                Transaction::Deposit(TxId(1), Amount(1.0)),
                Transaction::Withdrawal(TxId(2), Amount(2.0)),
                Transaction::Deposit(TxId(3), Amount(1.0)),
            ];

            let expected = AccountState {
                client_id,
                available: Amount(2.0),
                held: Amount(0.0),
                locked: false
            };

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }
    }


    #[test]
    fn run_test() {

        //no records
        {
            let records = vec![];
            let expected = vec![];

            let result = run(&records);

            assert_eq!(result, expected);
        }

        //one client + invalid record
        {
            let records = vec![
                InputRecord{r#type: "".to_string(), client: 1, tx: 1, amount: None},
                InputRecord{r#type: "deposit".to_string(), client: 1, tx: 2, amount: Some(10.0)},
                InputRecord{r#type: "withdrawal".to_string(), client: 1, tx: 3, amount: Some(2.0)},
            ];
            let expected = vec![
                AccountState{client_id: ClientId(1), available: Amount(8.0), held: Amount(0.0), locked: false},
            ];

            let result = run(&records);

            assert_eq!(result, expected);
        }

        //three clients + canceled overdrawing withdrawal
        {
            let records = vec![
                InputRecord{r#type: "deposit".to_string(), client: 1, tx: 616, amount: Some(10.0)},
                InputRecord{r#type: "deposit".to_string(), client: 2, tx: 525, amount: Some(10.0)},
                InputRecord{r#type: "deposit".to_string(), client: 3, tx: 434, amount: Some(10.0)},
                InputRecord{r#type: "withdrawal".to_string(), client: 3, tx: 343, amount: Some(2.0)},
                InputRecord{r#type: "withdrawal".to_string(), client: 2, tx: 252, amount: Some(8.0)},
                InputRecord{r#type: "withdrawal".to_string(), client: 1, tx: 161, amount: Some(15.0)},
            ];
            let expected = vec![
                AccountState{client_id: ClientId(1), available: Amount(10.0), held: Amount(0.0), locked: false},
                AccountState{client_id: ClientId(2), available: Amount(2.0), held: Amount(0.0), locked: false},
                AccountState{client_id: ClientId(3), available: Amount(8.0), held: Amount(0.0), locked: false},
            ];

            let result = run(&records);

            assert_eq!(result, expected);
        }
    }
}