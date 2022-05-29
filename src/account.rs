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
                    self.total -= tx.amount;
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
