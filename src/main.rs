use customer_account_provider::InMemoryCustomerAccountProvider;
use log::LevelFilter;

use log::{Level, Metadata, Record};
use transaction_history_provider::InMemoryTransactionHistoryProvider;
use transaction_requests_reader::TransactionRequestsReader;

use crate::transactions_manager::{TransactionsManager, DefaultTransactionsManager};

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

    let path = "/Users/kostard/Documents/Personal/projects/simple_payment_engine/test.csv";
    let reader = TransactionRequestsReader::new(path);
    let iterator = reader.read();
    let mut transactions_manager = DefaultTransactionsManager::new(
        InMemoryTransactionHistoryProvider::new(),
        InMemoryCustomerAccountProvider::new(),
    );
    iterator
        .filter(|request| DefaultTransactionsManager::structure_validation(request))
        .for_each(|request| {
            transactions_manager
                .handle_transaction(request)
                .expect("Something went wrong while handling the transaction")
        });
    println!(
        "Available: {}",
        transactions_manager
            .customer_account_provider
            .get_available(1)
            .unwrap()
            .unwrap()
    );
    println!(
        "Held: {}",
        transactions_manager
            .customer_account_provider
            .get_held_amount(1)
            .unwrap()
            .unwrap()
    );
    println!(
        "Locked: {}",
        transactions_manager
            .customer_account_provider
            .get_locked_status(1)
            .unwrap()
            .unwrap()
    );
    println!("{:?}", transactions_manager.transaction_history_provider.read_transaction_state(1));
}
