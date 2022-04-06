use csv::ReaderBuilder;

use crate::transaction_request::TransactionRequest;

pub struct TransactionRequestsReader<'a> {
    path: &'a str,
}

impl<'a> TransactionRequestsReader<'a> {
    pub fn new(path: &str) -> TransactionRequestsReader {
        TransactionRequestsReader { path }
    }

    pub fn read(self: &Self) -> impl Iterator<Item = TransactionRequest> {
        return ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b',')
            .trim(csv::Trim::All)
            .from_path(self.path.clone())
            .expect(&format!("Failed opening the file {}", self.path))
            .into_deserialize::<TransactionRequest>()
            .map(|record| record.expect("Failed extracting records"));
    }
}
