use crate::transaction_request::TransactionRequest;

pub trait TransactionHandler {
    fn structure_validation(transaction_request: &TransactionRequest) -> bool;
    fn handle_transaction(&mut self, transaction_request: TransactionRequest) -> Result<(), ()>;
}
