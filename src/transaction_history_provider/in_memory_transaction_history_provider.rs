use crate::{
    common_types::TransactionId,
    transaction_request::{TransactionRequest, TransactionState},
};
use std::collections::HashMap;

use super::transaction_history_provider::TransactionHistoryProvider;

pub struct InMemoryTransactionHistoryProvider {
    history: HashMap<TransactionId, TransactionRequest>,
    state: HashMap<TransactionId, TransactionState>,
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
    fn write_transaction(&mut self, transaction_request: TransactionRequest) -> Result<(), String> {
        // Maybe we can add transaction_id check here, to make sure no overrides happen
        self.history
            .insert(transaction_request.transaction_id, transaction_request);
        Ok(())
    }

    fn read_transaction(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<Option<&TransactionRequest>, String> {
        Ok(self.history.get(&transaction_id))
    }

    fn write_transaction_state(
        &mut self,
        transaction_id: TransactionId,
        transaction_state: TransactionState,
    ) -> Result<(), String> {
        self.state.insert(transaction_id, transaction_state);
        Ok(())
    }

    fn read_transaction_state(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<Option<&TransactionState>, String> {
        Ok(self.state.get(&transaction_id))
    }
}

#[cfg(test)]
mod in_memory_transaction_history_provider_tests {
    use rust_decimal::Decimal;

    use crate::transaction_request::TransactionType;

    use super::*;
    #[test]
    fn write_transaction_works_as_expected() {
        let mut transaction_history_provider = InMemoryTransactionHistoryProvider::new();
        let client_id = 1;
        let transaction_id = 1;
        let amount = Decimal::new(10, 0);
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        assert!(transaction_history_provider
            .write_transaction(transaction_request.clone())
            .is_ok());
        assert_eq!(
            transaction_history_provider.history.get(&transaction_id),
            Some(&transaction_request)
        );
    }
    #[test]
    fn read_transaction_works_as_expected() {
        let mut transaction_history_provider = InMemoryTransactionHistoryProvider::new();
        let client_id = 1;
        let transaction_id = 1;
        let amount = Decimal::new(10, 0);
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        assert!(transaction_history_provider
            .write_transaction(transaction_request.clone())
            .is_ok());
        assert_eq!(
            transaction_history_provider.read_transaction(transaction_id),
            Ok(Some(&transaction_request))
        );
    }

    #[test]
    fn write_transaction_state_works_as_expected() {
        let mut transaction_history_provider = InMemoryTransactionHistoryProvider::new();
        let transaction_id = 1;
        let transaction_state = TransactionState {
            held: true,
            charged_back: false,
        };
        assert!(transaction_history_provider
            .write_transaction_state(transaction_id, transaction_state.clone())
            .is_ok());
        assert_eq!(
            transaction_history_provider.state.get(&transaction_id),
            Some(&transaction_state)
        );
    }

    #[test]
    fn read_transaction_state_works_as_expected() {
        let mut transaction_history_provider = InMemoryTransactionHistoryProvider::new();
        let transaction_id = 1;
        let transaction_state = TransactionState {
            held: true,
            charged_back: false,
        };
        assert!(transaction_history_provider
            .write_transaction_state(transaction_id, transaction_state.clone())
            .is_ok());
        assert_eq!(
            transaction_history_provider.read_transaction_state(transaction_id),
            Ok(Some(&transaction_state))
        );
    }
}
