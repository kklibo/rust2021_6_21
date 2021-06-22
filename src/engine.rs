//! The transaction processing engine

use std::collections::{BTreeMap,HashMap,HashSet};
use std::error::Error;

use crate::input::InputRecord;
use crate::account_state::AccountState;


/// A client ID
#[derive(Copy,Clone,Eq,PartialEq,Ord,PartialOrd,Debug)]
pub struct ClientId(pub u16);

/// A globally-unique transaction ID
#[derive(Copy,Clone,Eq,PartialEq,Hash,Debug)]
struct TxId(u32);

/// A deposit or withdrawal amount; expected precision is 4 places past the decimal
#[derive(Copy,Clone,PartialEq,Debug)]
pub struct Amount(pub f32);

/// A transaction that applies to a client account
#[derive(PartialEq,Debug)]
enum Transaction {
    Deposit(TxId, Amount),
    Withdrawal(TxId, Amount),
    Dispute(TxId),
    Resolve(TxId),
    Chargeback(TxId),
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
        },

        InputRecord{r#type, client,tx, amount: None} => {

            match r#type.as_str() {
                "dispute"    => Ok((ClientId(*client), Transaction::Dispute(TxId(*tx)))),
                "resolve"    => Ok((ClientId(*client), Transaction::Resolve(TxId(*tx)))),
                "chargeback" => Ok((ClientId(*client), Transaction::Chargeback(TxId(*tx)))),
                _ => Err("invalid input record".into())
            }
        },

    }
}

/// Processes an account's transaction history and returns its current state.
/// Note: `client_id` is only used to create the AccountState:
/// all `transactions` will be processed.
fn process_account_transactions(client_id: ClientId, transactions: &Vec<Transaction>) -> Option<AccountState> {

    let mut account_state: Option<AccountState> = None;

    //for existing deposits: transaction IDs mapped to amounts
    let mut deposit_amounts = HashMap::<TxId, Amount>::new();

    //the transaction IDs of disputed deposits
    let mut disputed_deposit_ids = HashSet::<TxId>::new();

    for transaction in transactions {

        // Create the account state on the first deposit:
        // No other transactions are valid until the account is opened by a deposit.
        //
        // Note: this handling prevents an edge case bug in which an un-deposited account
        // could erroneously appear in the output after receiving non-deposit transactions:
        // such an account should be considered unopened, and therefore invalid.
        // In this function, a client account that never receives a deposit will return 'None'.
        if let None = account_state {
            if let Transaction::Deposit(_,_) = transaction {

                //this is the first deposit, so the account exists now
                account_state = Some(AccountState {
                    client_id,
                    available: Amount(0.0),
                    held: Amount(0.0),
                    locked: false
                });
            }
        }

        let account_state = match account_state {
            Some(ref mut a) => a,
            None => {
                //still waiting for the first deposit:
                // don't process this transaction, it predates its target account
                continue;
            }
        };

        match transaction {

            &Transaction::Deposit(tx_id, amount) => {

                //deposits always succeed
                account_state.available.0 += amount.0;

                //record this deposit, in case of a chargeback
                // note: this assumes transaction ID uniqueness: no check for insert() overwrite
                deposit_amounts.insert(tx_id, amount);
            },

            &Transaction::Withdrawal(_tx_id, amount) => {

                //withdrawals only happen if enough funds are available
                if account_state.available.0 >= amount.0 {
                    account_state.available.0 -= amount.0;
                }
            },

            &Transaction::Dispute(tx_id) => {

                //disputes only happen on existing deposits
                if let Some(&amount) = deposit_amounts.get(&tx_id) {

                    //hold the disputed funds
                    account_state.available.0 -= amount.0;
                    account_state.held.0 += amount.0;

                    //record the disputed status
                    // note: this assumes transaction ID uniqueness: no check for insert() overwrite
                    disputed_deposit_ids.insert(tx_id);
                }
            },

            &Transaction::Resolve(tx_id) => {

                //resolve only applies to an existing disputed deposit
                if let Some(&amount) = deposit_amounts.get(&tx_id) {

                    if disputed_deposit_ids.contains(&tx_id) {

                        //make the disputed funds available
                        account_state.available.0 += amount.0;
                        account_state.held.0 -= amount.0;
                    }

                    //remove the disputed status
                    disputed_deposit_ids.remove(&tx_id);
                }
            },

            &Transaction::Chargeback(tx_id) => {

                //chargeback only applies to an existing disputed deposit
                if let Some(&amount) = deposit_amounts.get(&tx_id) {

                    if disputed_deposit_ids.contains(&tx_id) {

                        //remove the chargeback withdrawal from held funds
                        account_state.held.0 -= amount.0;

                        //lock (also "freeze") this account
                        account_state.locked = true;

                        //now that the client account is locked, no more actions are possible:
                        //ignore all remaining transactions
                        break;
                    }
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
    let mut account_histories = BTreeMap::<ClientId, Vec<Transaction>>::new();

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
    account_histories.iter().filter_map(|(&client_id, transactions)| {
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

        //success: dispute
        {
            let record = InputRecord{r#type: "dispute".to_string(), client: 1, tx: 2, amount: None};
            let result = parse_record(&record).unwrap();

            assert_eq!(result, (ClientId(1), Transaction::Dispute(TxId(2))));
        }

        //success: resolve
        {
            let record = InputRecord{r#type: "resolve".to_string(), client: 1, tx: 2, amount: None};
            let result = parse_record(&record).unwrap();

            assert_eq!(result, (ClientId(1), Transaction::Resolve(TxId(2))));
        }

        //success: chargeback
        {
            let record = InputRecord{r#type: "chargeback".to_string(), client: 1, tx: 2, amount: None};
            let result = parse_record(&record).unwrap();

            assert_eq!(result, (ClientId(1), Transaction::Chargeback(TxId(2))));
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

        //failure: dispute has an amount
        {
            let record = InputRecord{r#type: "dispute".to_string(), client: 1, tx: 2, amount: Some(3.0)};
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

            let expected = None;

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

            let expected = Some(AccountState {
                client_id,
                available: Amount(10.0),
                held: Amount(0.0),
                locked: false
            });

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

            let expected = Some(AccountState {
                client_id,
                available: Amount(2.0),
                held: Amount(0.0),
                locked: false
            });

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }

        //pending dispute (neither resolved nor charged back)
        {
            let transactions = vec![
                Transaction::Deposit(TxId(1), Amount(10.0)),
                Transaction::Dispute(TxId(1)),
            ];

            let expected = Some(AccountState {
                client_id,
                available: Amount(0.0),
                held: Amount(10.0),
                locked: false
            });

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }

        //resolved dispute
        {
            let transactions = vec![
                Transaction::Deposit(TxId(1), Amount(10.0)),
                Transaction::Dispute(TxId(1)),
                Transaction::Resolve(TxId(1)),
            ];

            let expected = Some(AccountState {
                client_id,
                available: Amount(10.0),
                held: Amount(0.0),
                locked: false
            });

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }

        //chargeback with blocked subsequent transaction attempts
        {
            let transactions = vec![
                Transaction::Deposit(TxId(1), Amount(10.0)),
                Transaction::Dispute(TxId(1)),
                Transaction::Chargeback(TxId(1)),

                //remaining transactions will not happen (account is locked/frozen)
                Transaction::Resolve(TxId(1)),
                Transaction::Deposit(TxId(1), Amount(100.0)),
                Transaction::Withdrawal(TxId(1), Amount(5.0)),
            ];

            let expected = Some(AccountState {
                client_id,
                available: Amount(0.0),
                held: Amount(0.0),
                locked: true
            });

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }

        //dispute resolution precedes deposit
        {
            let transactions = vec![
                //these transactions have no effect, their target doesn't exist yet
                Transaction::Dispute(TxId(1)),
                Transaction::Chargeback(TxId(1)),
                Transaction::Resolve(TxId(1)),
                //

                Transaction::Deposit(TxId(1), Amount(10.0)),
            ];

            let expected = Some(AccountState {
                client_id,
                available: Amount(10.0),
                held: Amount(0.0),
                locked: false
            });

            let result = process_account_transactions(client_id, &transactions);

            assert_eq!(result, expected);
        }

        //dispute resolution precedes dispute
        {
            let transactions = vec![
                Transaction::Deposit(TxId(1), Amount(10.0)),

                //these transactions have no effect, their target isn't disputed yet
                Transaction::Chargeback(TxId(1)),
                Transaction::Resolve(TxId(1)),
                //

                Transaction::Dispute(TxId(1)),
            ];

            let expected = Some(AccountState {
                client_id,
                available: Amount(0.0),
                held: Amount(10.0),
                locked: false
            });

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

        //three clients w/ disputes: pending, resolved, and charged back
        {
            let records = vec![
                InputRecord{r#type: "deposit".to_string(), client: 1, tx: 616, amount: Some(10.0)},
                InputRecord{r#type: "deposit".to_string(), client: 2, tx: 525, amount: Some(10.0)},
                InputRecord{r#type: "deposit".to_string(), client: 3, tx: 434, amount: Some(10.0)},

                InputRecord{r#type: "dispute".to_string(), client: 1, tx: 616, amount: None},
                InputRecord{r#type: "dispute".to_string(), client: 2, tx: 525, amount: None},
                InputRecord{r#type: "dispute".to_string(), client: 3, tx: 434, amount: None},

                InputRecord{r#type: "resolve".to_string(), client: 2, tx: 525, amount: None},
                InputRecord{r#type: "chargeback".to_string(), client: 3, tx: 434, amount: None},

                InputRecord{r#type: "withdrawal".to_string(), client: 3, tx: 343, amount: Some(5.0)},
                InputRecord{r#type: "withdrawal".to_string(), client: 2, tx: 252, amount: Some(5.0)},
                InputRecord{r#type: "withdrawal".to_string(), client: 1, tx: 161, amount: Some(5.0)},
            ];

            let expected = vec![
                AccountState{client_id: ClientId(1), available: Amount(0.0), held: Amount(10.0), locked: false},
                AccountState{client_id: ClientId(2), available: Amount(5.0), held: Amount(0.0), locked: false},
                AccountState{client_id: ClientId(3), available: Amount(0.0), held: Amount(0.0), locked: true},
            ];

            let result = run(&records);

            assert_eq!(result, expected);
        }

        //disputes + resolutions: wrong clients/transaction IDs
        {
            let records = vec![
                InputRecord{r#type: "deposit".to_string(), client: 1, tx: 616, amount: Some(10.0)},
                InputRecord{r#type: "deposit".to_string(), client: 2, tx: 525, amount: Some(10.0)},
                InputRecord{r#type: "deposit".to_string(), client: 3, tx: 434, amount: Some(10.0)},

                //wrong client
                InputRecord{r#type: "dispute".to_string(), client: 2, tx: 616, amount: None},

                //non-existent client
                InputRecord{r#type: "dispute".to_string(), client: 5, tx: 525, amount: None},

                //valid
                InputRecord{r#type: "dispute".to_string(), client: 3, tx: 434, amount: None},

                //wrong transaction ID
                InputRecord{r#type: "resolve".to_string(), client: 2, tx: 434, amount: None},

                //wrong client
                InputRecord{r#type: "chargeback".to_string(), client: 2, tx: 434, amount: None},
            ];

            let expected = vec![
                AccountState{client_id: ClientId(1), available: Amount(10.0), held: Amount(0.0), locked: false},
                AccountState{client_id: ClientId(2), available: Amount(10.0), held: Amount(0.0), locked: false},
                AccountState{client_id: ClientId(3), available: Amount(0.0), held: Amount(10.0), locked: false},
            ];

            let result = run(&records);

            assert_eq!(result, expected);
        }
    }
}