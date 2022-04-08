use std::env::args;

use customer_account_provider::InMemoryCustomerAccountProvider;
use log::{info, LevelFilter};

use log::{Level, Metadata, Record};
use transaction_history_provider::InMemoryTransactionHistoryProvider;
use transaction_requests_reader::{TransactionRequestsReader, DefaultTransactionRequestsReader, DummyReader};

use crate::transactions_manager::{DefaultTransactionsManager, TransactionsManager};

mod common_types;
mod customer_account_provider;
mod transaction_history_provider;
mod transaction_request;
mod transaction_requests_reader;
mod transactions_manager;

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();

    // let path = match args().nth(1) {
    //     Some(e) => e,
    //     None => panic!("Path not passed for the input file!")
    // };

    // let reader = DefaultTransactionRequestsReader::new(&path);
    let reader = DummyReader {};
    let iterator = reader.read();
    let mut transactions_manager = DefaultTransactionsManager::new(
        InMemoryTransactionHistoryProvider::new(),
        InMemoryCustomerAccountProvider::new(),
    );
    iterator
        .filter(|request| DefaultTransactionsManager::structure_validation(request))
        .for_each(|request| {
            if !transactions_manager
                .handle_transaction(request)
                .expect("Something went wrong while handling the transaction")
            {
                info!("Request skipped");
            }
        });
    transactions_manager.print_report().expect("Printing the report failed.");
}
