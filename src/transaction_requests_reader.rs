use csv::ReaderBuilder;
use log::info;
use rust_decimal::Decimal;

use crate::transaction_request::{TransactionRequest, TransactionType};

pub trait TransactionRequestsReader {
    fn read(self: &Self) -> Box<dyn Iterator<Item = TransactionRequest>>;
}

pub struct DefaultTransactionRequestsReader {
    path: String,
    enforced_scale: u32,
}

impl DefaultTransactionRequestsReader {
    pub fn new(path: &str) -> DefaultTransactionRequestsReader {
        DefaultTransactionRequestsReader {
            path: path.to_owned(),
            enforced_scale: 4,
        }
    }
}

impl TransactionRequestsReader for DefaultTransactionRequestsReader {
    fn read(self: &Self) -> Box<dyn Iterator<Item = TransactionRequest>> {
        let path = self.path.clone();
        let enforced_scale = self.enforced_scale;
        return Box::new(
            ReaderBuilder::new()
                .has_headers(true)
                .delimiter(b',')
                .trim(csv::Trim::All)
                .from_path(path.clone())
                .expect(&format!("Failed opening the file {}", path))
                .into_deserialize::<TransactionRequest>()
                .map(|record| record.expect("Failed extracting records"))
                .map(move |record| {
                    if let Some(mut amount) = record.amount {
                        if amount.scale() > enforced_scale {
                            info!("Scaling down the decimal - {}", amount);
                            amount
                                .set_scale(amount.scale() - enforced_scale)
                                .expect("Couldn't change the amount scale.");
                            let mut new_amount = amount.trunc();
                            new_amount
                                .set_scale(enforced_scale)
                                .expect("Couldn't change the new amount scale.");
                            return TransactionRequest {
                                amount: Some(new_amount),
                                ..record
                            };
                        }
                    }
                    record
                }),
        );
    }
}

pub struct DummyReader;

// For stress testing
impl TransactionRequestsReader for DummyReader {
    fn read(self: &Self) -> Box<dyn Iterator<Item = TransactionRequest>> {
        return Box::new((1..=u32::MAX).map(|i| TransactionRequest {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: i,
            amount: Some(Decimal::new(10, 0)),
        }));
    }
}

#[cfg(test)]
mod default_transaction_requests_reader {
    use crate::transaction_request::TransactionType;
    use std::io::Write;

    use super::*;

    use rust_decimal::Decimal;
    use tempfile::{NamedTempFile, TempPath};

    #[test]
    fn read_works_as_expected() {
        let content = "
        type, client, tx, amount
        deposit, 1, 1, 10.2
        withdrawal, 1, 2, 10.3
        dispute, 1, 1, 
        resolve, 1, 1, 
        dispute, 1, 1,
        chargeback, 1, 1, ";
        let path = save_to_temp_file(content);
        let transaction_requests_reader =
            DefaultTransactionRequestsReader::new(path.to_str().unwrap());
        let records: Vec<TransactionRequest> = transaction_requests_reader.read().collect();
        assert_eq!(
            records,
            vec![
                TransactionRequest {
                    transaction_type: TransactionType::Deposit,
                    client_id: 1,
                    transaction_id: 1,
                    amount: Some(Decimal::new(102, 1))
                },
                TransactionRequest {
                    transaction_type: TransactionType::Withdrawal,
                    client_id: 1,
                    transaction_id: 2,
                    amount: Some(Decimal::new(103, 1))
                },
                TransactionRequest {
                    transaction_type: TransactionType::Dispute,
                    client_id: 1,
                    transaction_id: 1,
                    amount: None
                },
                TransactionRequest {
                    transaction_type: TransactionType::Resolve,
                    client_id: 1,
                    transaction_id: 1,
                    amount: None
                },
                TransactionRequest {
                    transaction_type: TransactionType::Dispute,
                    client_id: 1,
                    transaction_id: 1,
                    amount: None
                },
                TransactionRequest {
                    transaction_type: TransactionType::Chargeback,
                    client_id: 1,
                    transaction_id: 1,
                    amount: None
                }
            ]
        );
        path.close().unwrap();
    }

    #[test]
    fn read_enforces_the_decimal_scale() {
        let content = "
        type, client, tx, amount
        deposit, 1, 1, 10.23456";
        let path = save_to_temp_file(content);
        let transaction_requests_reader =
            DefaultTransactionRequestsReader::new(path.to_str().unwrap());
        let records: Vec<TransactionRequest> = transaction_requests_reader.read().collect();
        assert_eq!(
            records,
            vec![TransactionRequest {
                transaction_type: TransactionType::Deposit,
                client_id: 1,
                transaction_id: 1,
                amount: Some(Decimal::new(102345, 4))
            }]
        );
        path.close().unwrap();
    }

    fn save_to_temp_file(content: &str) -> TempPath {
        let mut file = NamedTempFile::new().expect("Couldn't create a temporary file for testing");
        file.write_all(content.as_bytes())
            .expect("Couldn't write into the temp file for unit-testing");
        return file.into_temp_path();
    }
}
