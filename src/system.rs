use crate::account::AccountState;
use crate::transaction::Transaction;
use hashring::HashRing;
use std::cell::Cell;
use std::collections::HashMap;

pub struct AccountSystem {
    accounts: Cell<HashMap<u16, AccountState>>,
}

impl AccountSystem {
    pub fn new() -> Self {
        AccountSystem {
            accounts: Cell::new(HashMap::new()),
        }
    }

    pub fn transact(&mut self, transaction: Transaction) {
        if let Some(account) = self.accounts.get_mut().get_mut(transaction.id()) {
            account.transact(transaction)
        } else {
            let id = *transaction.id();
            let mut account = AccountState::new();
            account.transact(transaction);
            self.accounts.get_mut().insert(id, account);
        }
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
}
