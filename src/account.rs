use crate::Transaction;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Apart from the amount of the deposit, a deposit could be disputed as well as
/// it could be linked to a chargeback. It is easy to store that state in a structure
/// private to the module for convenience.
#[derive(Debug, Copy, Clone, Hash, PartialEq)]
pub struct DepositState {
    amount: Decimal,
    dispute: bool,
    chargeback: bool,
}

impl DepositState {
    /// A simple constructor. Serves no other purpose than convenience.
    fn new(amount: Decimal) -> Self {
        DepositState {
            amount,
            dispute: false,
            chargeback: false,
        }
    }
}

/// At any given point an account will have a state that is represented by this structure.
/// In a real world application, this will likely be backed by a persistent data store,
/// but for our demo purposes that is not strictly necessary.
/// Generally speaking, without knowing much else about the problem if I had more time I would
/// have gone ahead and stored this in an RDBMS. The benefits of that are that many of
/// the calculations can be done as a complex SQL query without any need for network I/O between
/// database an application code.
pub struct AccountState {
    pub held: Decimal,
    pub total: Decimal,
    pub chargebacks: u32,
    pub deposits: HashMap<u32, DepositState>,
}

impl AccountState {
    pub fn available(&self) -> Decimal {
        self.total - self.held
    }

    pub fn locked(&self) -> bool {
        self.chargebacks != 0
    }

    pub fn new() -> Self {
        AccountState {
            held: Decimal::zero(),
            total: Decimal::zero(),
            chargebacks: 0,
            deposits: HashMap::new(),
        }
    }

    /// Few things to add:
    /// 1. There are more than one ways to think about chargebacks. These are the assumptions we're making:
    ///     a) More than one transaction can have a chargeback. Think of more than one transaction being
    ///         disputed and then reversed. That will be a double chargeback. We consider them all by
    ///         marking that in the deposit state.
    ///     b) We could have also used `chargebacks` as a vector of deposit IDs and identified the lock status
    ///         of an account based on the count. We just maintain a counter and mark the individual deposits
    ///         instead. There is little difference between the two, so I went with my first instinct.
    ///
    /// 2. Several style guides will argue against the early return pattern. Google's style-guide is one that
    ///     says that early returns are good. Like all interesting problems -- I'd say, it depends. I'm using
    ///     early returns here because the code is likely not going to get too big and this appears to be
    ///     well readable.
    pub fn transact(&mut self, transaction: Transaction) {
        match transaction {
            Transaction::Deposit { tx, amount, .. } => {
                if self.locked() {
                    return;
                }
                self.total += amount;
                self.deposits.insert(tx, DepositState::new(amount));
            }
            Transaction::Withdrawal { amount, .. } => {
                if self.locked() {
                    return;
                }
                if self.available() > amount {
                    self.total -= amount;
                }
            }
            Transaction::Dispute { tx, .. } => {
                if let Some(tx) = self.deposits.get_mut(&tx) {
                    tx.dispute = true;
                    self.held += tx.amount;
                }
            }
            Transaction::Resolve { tx, .. } => {
                if let Some(tx) = self.deposits.get_mut(&tx) {
                    tx.dispute = false;
                    self.total += tx.amount;
                    self.held -= tx.amount;
                }
            }
            Transaction::Chargeback { tx, .. } => {
                if let Some(tx) = self.deposits.get_mut(&tx) {
                    if tx.dispute {
                        tx.chargeback = true;
                        self.chargebacks += 1;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that basic deposit and withdraw works
    fn deposit_withdraw_test() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        // Deposit was successful
        assert_eq!(state.available(), Decimal::from(100));
        state.transact(Transaction::Withdrawal {
            client: 0,
            tx: 1,
            amount: Decimal::from(50),
        });
        // Withdraw successful
        assert_eq!(state.available(), Decimal::from(50));
    }

    #[test]
    /// Fail withdrawal when not enough funds are available
    fn fail_withdraw_no_funds() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        // Deposit was successful
        assert_eq!(state.available(), Decimal::from(100));
        state.transact(Transaction::Withdrawal {
            client: 0,
            tx: 1,
            amount: Decimal::from(150),
        });
        // Withdraw failure
        assert_eq!(state.available(), Decimal::from(100));
    }

    #[test]
    /// A broken transaction should not brake the state management
    fn success_successive_no_transactions_after_failure() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        // Deposit was successful
        assert_eq!(state.available(), Decimal::from(100));
        state.transact(Transaction::Withdrawal {
            client: 0,
            tx: 1,
            amount: Decimal::from(150),
        });
        // Withdraw failure
        assert_eq!(state.available(), Decimal::from(100));
        state.transact(Transaction::Withdrawal {
            client: 0,
            tx: 1,
            amount: Decimal::from(50),
        });
        // Withdraw success
        assert_eq!(state.available(), Decimal::from(50));
    }

    #[test]
    /// Basic dispute management
    fn manage_disputes() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 1,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Dispute { client: 0, tx: 1 });
        assert_eq!(state.available(), Decimal::from(100));
    }

    #[test]
    /// If a transaction doesn't exist, ignore disputes
    fn no_dispute_bad_tx() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 1,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Dispute { client: 0, tx: 3 });
        assert_eq!(state.available(), Decimal::from(200));
    }

    #[test]
    /// More than one dispute is possible
    fn manage_multiple_disputes() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        // Deposit was successful
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 1,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 2,
            amount: Decimal::from(200),
        });
        state.transact(Transaction::Dispute { client: 0, tx: 0 });
        state.transact(Transaction::Dispute { client: 0, tx: 1 });
        assert_eq!(state.available(), Decimal::from(200));
    }

    #[test]
    /// If a transaction is not disputed, chargeback should fail
    fn no_dispute_no_chargeback() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        // Deposit was successful
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 1,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Chargeback { client: 0, tx: 1 });
        assert!(!state.locked());
    }

    #[test]
    /// Account should be locked if a chargeback happens
    fn chargeback() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        // Deposit was successful
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 1,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Dispute { client: 0, tx: 1 });
        state.transact(Transaction::Chargeback { client: 0, tx: 1 });
        assert!(state.locked());
    }

    #[test]
    /// Deposits should be possible, but withdrawal not, after a chargeback
    fn no_further_withdraw_after_chargeback() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 1,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Dispute { client: 0, tx: 1 });
        state.transact(Transaction::Chargeback { client: 0, tx: 1 });
        assert!(state.locked());
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 2,
            amount: Decimal::from(100),
        });
        // I'm assuming that funds still show up as available even if withdrawal fails
        assert_eq!(state.available(), Decimal::from(100));
        state.transact(Transaction::Withdrawal {
            client: 0,
            tx: 2,
            amount: Decimal::from(100),
        });
        assert_eq!(state.available(), Decimal::from(100));
    }

    #[test]
    /// A chargeback doesn't mean that further disputes aren't possible
    fn disputes_possible_after_chargeback() {
        let mut state = AccountState::new();
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 0,
            amount: Decimal::from(100),
        });
        // Deposit was successful
        state.transact(Transaction::Deposit {
            client: 0,
            tx: 1,
            amount: Decimal::from(100),
        });
        state.transact(Transaction::Dispute { client: 0, tx: 1 });
        state.transact(Transaction::Chargeback { client: 0, tx: 1 });
        assert!(state.locked());
        state.transact(Transaction::Dispute { client: 0, tx: 0 });
        assert_eq!(state.available(), Decimal::from(0));
        assert!(state.locked()); // Still locked
    }
}
