use crate::account::AccountState;
use crate::transaction::Transaction;
use crate::Output;
use csv::Writer;
use hashring::HashRing;
use std::collections::HashMap;
use std::io::Stdout;

/// Think of this as a database (or rather a key-value store) that can be used to
/// store more than one [AccountState].
/// One can call [AccountSystem::transact] to run a specific transaction for a given
/// user account.
pub struct AccountSystem {
    /// A HashMap is probably the best structure for in-memory calculation
    /// because we need to frequently look for accounts using the ID.
    /// This will yield a constant time lookup, which is probably the best we can do.
    accounts: HashMap<u16, AccountState>,
}

impl AccountSystem {
    /// Nothing fancy. Just a nice-to-have constructor.
    pub fn new() -> Self {
        AccountSystem {
            accounts: HashMap::new(),
        }
    }

    /// Let's apply a transaction to an account in our register.
    /// If such an account does not exist, we initialise an empty account.
    pub fn transact(&mut self, transaction: Transaction) {
        if let Some(account) = self.accounts.get_mut(transaction.id()) {
            account.transact(transaction)
        } else {
            let id = *transaction.id();
            let mut account = AccountState::new();
            account.transact(transaction);
            self.accounts.insert(id, account);
        }
    }

    /// We simply write the CSV content out to write-buffer based on the current account state
    pub fn write(&self, writer: &mut Writer<Stdout>) -> std::io::Result<()> {
        for (client, account) in self.accounts.iter() {
            writer.serialize(Output {
                client: *client,
                available: account.available(),
                held: account.held,
                total: account.total,
                locked: account.locked(),
            })?;
        }
        Ok(())
    }
}

/// The problem statement calls for consideration for a real-world case where the input
/// can be streamed and tasks executed more efficiently.
/// The good thing about working with multiple objects (users) is that we can shard them
/// by key and we only have to ensure that the order is maintained across a single user.
/// This means we can run transactions in parallel for various users.
///
/// We do not implement any sophisticated database semantics, instead we "simulate" sharding
/// behaviour by using the user's ID as a shard-key and multiple account-systems which then
/// execute these requests serially.
pub struct ShardedAccountSystem {
    ring: HashRing<usize>,
    systems: Vec<AccountSystem>,
}

impl ShardedAccountSystem {
    /// It's always nice to be able to decide on the level of parallelism based
    /// on other constraints (i.e., CPU, network, etc.). So we allow one to
    /// create a select number of shards when they initiate this sytem.
    pub fn new(shards: usize) -> Self {
        let mut ring = HashRing::new();
        let mut systems = Vec::new();
        for shard in 0..shards {
            systems.push(AccountSystem::new());
            ring.add(shard);
        }
        ShardedAccountSystem { ring, systems }
    }

    /// This could very well be executed in parallel with individual account-systems executing
    /// their tasks using a Tokio Task and using a channel to dispatch information to them.
    /// Unfortunately, I did not have time to implement that, so I stopped at "simulating"
    /// similar behaviour using a sync API.
    /// Of course, in a real-world application, the entire point of sharded-transaction systems is
    /// lost without an async API, and I would have done this differently had this been a production
    /// application or if I had had more time.
    pub fn transact(&mut self, transaction: Transaction) {
        let id = *transaction.id();
        if let Some(shard) = self.ring.get(&id.to_be_bytes()) {
            self.systems[*shard].transact(transaction);
        }
    }

    /// While we're calling the same write function as that of contained [AccountSystem],
    /// we flush the buffer after every shard in case they start getting too big.
    /// Of course, this is not very likely for our application because everything is in memory
    /// nevertheless, but it's definitely nice to consider that for extreme cases.
    pub fn write(&self, writer: &mut Writer<Stdout>) -> std::io::Result<()> {
        for system in self.systems.iter() {
            system.write(writer)?;
            writer.flush()?;
        }
        Ok(())
    }
}
