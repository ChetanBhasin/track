use crate::account::AccountState;
use crate::transaction::Transaction;
use crate::Output;
use csv::Writer;
use hashring::HashRing;
use std::collections::HashMap;
use std::io::Stdout;

pub struct AccountSystem {
    accounts: HashMap<u16, AccountState>,
}

impl AccountSystem {
    pub fn new() -> Self {
        AccountSystem {
            accounts: HashMap::new(),
        }
    }

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

pub struct ShardedAccountSystem {
    ring: HashRing<usize>,
    systems: Vec<AccountSystem>,
}

impl ShardedAccountSystem {
    pub fn new(shards: usize) -> Self {
        let mut ring = HashRing::new();
        let mut systems = Vec::new();
        for shard in 0..shards {
            systems.push(AccountSystem::new());
            ring.add(shard);
        }
        ShardedAccountSystem { ring, systems }
    }

    pub fn transact(&mut self, transaction: Transaction) {
        let id = *transaction.id();
        if let Some(shard) = self.ring.get(&id.to_be_bytes()) {
            self.systems[*shard].transact(transaction);
        }
    }

    pub fn write(&self, writer: &mut Writer<Stdout>) -> std::io::Result<()> {
        for system in self.systems.iter() {
            system.write(writer)?;
        }
        Ok(())
    }
}
