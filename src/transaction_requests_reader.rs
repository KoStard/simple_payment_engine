use csv::ReaderBuilder;
use log::info;

use crate::transaction_request::TransactionRequest;

pub struct TransactionRequestsReader<'a> {
    path: &'a str,
    enforced_scale: u32
}

impl<'a> TransactionRequestsReader<'a> {
    pub fn new(path: &str) -> TransactionRequestsReader {
        TransactionRequestsReader { path, enforced_scale: 4 }
    }

    pub fn read(self: &Self) -> impl Iterator<Item = TransactionRequest> + '_ {
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
                    if amount.scale() > self.enforced_scale {
                        info!("Scaling down the decimal - {}", amount);
                        amount.set_scale(amount.scale() - self.enforced_scale).expect("Couldn't change the amount scale.");
                        let mut new_amount = amount.trunc();
                        // TODO test
                        new_amount.set_scale(self.enforced_scale).expect("Couldn't change the new amount scale.");
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
