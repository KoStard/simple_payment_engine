use std::collections::HashMap;

use crate::transaction_request::{TransactionRequest, TransactionState};

// TODO Add explanation why using Result
pub trait TransactionHistoryProvider {
    fn write_transaction(&mut self, transaction_request: TransactionRequest) -> Result<(), ()>;
    fn read_transaction(&mut self, transaction_id: u32) -> Result<Option<&TransactionRequest>, ()>;
    fn write_transaction_state(
        &mut self,
        transaction_id: u32,
        transaction_state: TransactionState,
    ) -> Result<(), ()>;
    fn read_transaction_state(
        &mut self,
        transaction_id: u32,
    ) -> Result<Option<&TransactionState>, ()>;
}

pub struct InMemoryTransactionHistoryProvider {
    history: HashMap<u32, TransactionRequest>,
    state: HashMap<u32, TransactionState>,
}

impl InMemoryTransactionHistoryProvider {
    pub fn new() -> Self {
        InMemoryTransactionHistoryProvider {
            history: HashMap::new(),
            state: HashMap::new(),
        }
    }
}

impl TransactionHistoryProvider for InMemoryTransactionHistoryProvider {
    fn write_transaction(&mut self, transaction_request: TransactionRequest) -> Result<(), ()> {
        self.history
            .insert(transaction_request.transaction_id, transaction_request);
        Ok(())
    }

    fn read_transaction(&mut self, transaction_id: u32) -> Result<Option<&TransactionRequest>, ()> {
        Ok(self.history.get(&transaction_id))
    }

    fn write_transaction_state(
        &mut self,
        transaction_id: u32,
        transaction_state: TransactionState,
    ) -> Result<(), ()> {
        self.state.insert(transaction_id, transaction_state);
        Ok(())
    }

    fn read_transaction_state(
        &mut self,
        transaction_id: u32,
    ) -> Result<Option<&TransactionState>, ()> {
        Ok(self.state.get(&transaction_id))
    }
}
