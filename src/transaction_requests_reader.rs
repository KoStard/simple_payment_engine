use csv::ReaderBuilder;
use log::info;

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
            .map(|record| record.expect("Failed extracting records"))
            .map(|record| {
                if let Some(mut amount) = record.amount {
                    if amount.scale() > 4 {
                        info!("Scaling down the decimal - {}", amount);
                        amount.set_scale(amount.scale() - 4).expect("Couldn't change the amount scale.");
                        let mut new_amount = amount.trunc();
                        // TODO test
                        new_amount.set_scale(4).expect("Couldn't change the new amount scale.");
                        return TransactionRequest {
                            amount: Some(new_amount),
                            ..record
                        }
                    }
                }
                record
            });
    }
}
