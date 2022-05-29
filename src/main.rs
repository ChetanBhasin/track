mod account;
mod system;
mod transaction;

use crate::system::ShardedAccountSystem;
use crate::transaction::Transaction;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::{env, io};

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

#[derive(Serialize)]
struct Output {
    pub client: u16,
    #[serde(with = "rust_decimal::serde::float")]
    pub available: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub held: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub total: Decimal,
    pub locked: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let filepath = &args[1];
    let file = File::open(filepath.as_str())?;
    let reader = BufReader::new(file);

    let mut rdr = csv::Reader::from_reader(reader);
    let mut system = ShardedAccountSystem::new(3);
    let mut wtr = csv::Writer::from_writer(io::stdout());

    for result in rdr.deserialize() {
        let record: Input = result?;
        system.transact(record.try_into()?);
    }

    system.write(&mut wtr)?;
    let _ = wtr.flush()?;
    Ok(())
}
