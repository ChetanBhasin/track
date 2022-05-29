use crate::Input;
use anyhow::bail;
use rust_decimal::Decimal;
use std::convert::TryInto;

/// We want to ensure that the incoming transactions are valid and as such it is useful to
/// wrap them into their own discriminated union for both validation and convenience of
/// discrimination for further use.
pub enum Transaction {
    Deposit {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Withdrawal {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Dispute {
        client: u16,
        tx: u32,
    },
    Resolve {
        client: u16,
        tx: u32,
    },
    Chargeback {
        client: u16,
        tx: u32,
    },
}

impl Transaction {
    pub fn id(&self) -> &u16 {
        match self {
            Self::Deposit { client, .. } => client,
            Self::Withdrawal { client, .. } => client,
            Self::Dispute { client, .. } => client,
            Self::Resolve { client, .. } => client,
            Self::Chargeback { client, .. } => client,
        }
    }
}

impl TryInto<Transaction> for Input {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Transaction, Self::Error> {
        match self.type_.as_str() {
            "deposit" => Ok(Transaction::Deposit {
                client: self.client,
                tx: self.tx,
                amount: self
                    .amount
                    .expect("An amount needs to be specified for deposit.")
                    .round_dp(4), // Round to 4 decimal places
            }),
            "withdrawal" => Ok(Transaction::Withdrawal {
                client: self.client,
                tx: self.tx,
                amount: self
                    .amount
                    .expect("An amount needs to be specified for withdraw.")
                    .round_dp(4), // Round to 4 decimal places
            }),
            "dispute" => Ok(Transaction::Dispute {
                client: self.client,
                tx: self.tx,
            }),
            "resolve" => Ok(Transaction::Resolve {
                client: self.client,
                tx: self.tx,
            }),
            "chargeback" => Ok(Transaction::Chargeback {
                client: self.client,
                tx: self.tx,
            }),
            // Based on our handling, this will stop the program. However, IMHO, it should stop because
            // this probably means something terrible has happened and continuing process is unlikely
            // to yield correct state in the end.
            _ => bail!("Following input could not be parsed: {:?}", self),
        }
    }
}
