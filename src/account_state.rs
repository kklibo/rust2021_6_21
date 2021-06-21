use std::fmt::{Display, Formatter};

use crate::engine::{ClientId,Amount};

/// The state of a client account, `Display`-able as an output CSV line
#[derive(PartialEq,Debug)]
pub struct AccountState {

    ///client ID
    pub client_id: ClientId,
    ///Total undisputed funds
    pub available: Amount,
    ///Total disputed funds
    pub held: Amount,
    ///true IFF a chargeback has been issued on this account
    pub locked: bool,
}

impl Display for AccountState {

    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        let total = self.available.0 + self.held.0;

        //CSV output line format:
        // client, available, held, total, locked

        write!(f, "{},{:.4},{:.4},{:.4},{}",
            self.client_id.0, self.available.0, self.held.0, total, self.locked
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn display_test() {

        //success
        {
            let account = AccountState {
                client_id: ClientId(1), available: Amount(2.00), held: Amount(3.0), locked: false
            };
            assert_eq!(account.to_string(), "1,2.0000,3.0000,5.0000,false");
        }

        //success with float output truncation
        {
            let account = AccountState {
                client_id: ClientId(1), available: Amount(2.12341234), held: Amount(3.0), locked: true
            };
            assert_eq!(account.to_string(), "1,2.1234,3.0000,5.1234,true");
        }
    }
}