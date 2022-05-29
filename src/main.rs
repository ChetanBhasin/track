mod account;
mod system;
mod transaction;

use crate::system::ShardedAccountSystem;
use crate::transaction::Transaction;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
struct Input {
    #[serde(rename = "type")]
    type_: String,
    client: u16,
    tx: u32,
    // Since we want to manage a specific precision, we are going to use the decimal
    // crate to ease our workload.
    amount: Option<Decimal>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let filepath = &args[1];
    let file = File::open(filepath.as_str())?;
    let reader = BufReader::new(file);

    let mut rdr = csv::Reader::from_reader(reader);
    let mut system = ShardedAccountSystem::new(3);

    for result in rdr.deserialize() {
        let record: Input = result?;
        system.transact(record.try_into()?);
    }

    Ok(())
}
