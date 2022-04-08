use csv::WriterBuilder;
use mockall::predicate::*;
use mockall::*;
use rust_decimal::Decimal;

use crate::{
    common_types::TransactionId,
    customer_account_provider::CustomerAccountProvider,
    transaction_history_provider::transaction_history_provider::TransactionHistoryProvider,
    transaction_request::{TransactionRequest, TransactionState, TransactionType},
};

use log::info;

#[automock]
pub trait TransactionsManager {
    fn structure_validation(transaction_request: &TransactionRequest) -> bool;
    // Returning bool for showing if the transaction was executed
    fn handle_transaction(
        &mut self,
        transaction_request: TransactionRequest,
    ) -> Result<bool, String>;
    fn print_report(&self) -> Result<(), String>;
}

pub struct DefaultTransactionsManager {
    transaction_history_provider: Box<dyn TransactionHistoryProvider>,
    customer_account_provider: Box<dyn CustomerAccountProvider>,
}
impl DefaultTransactionsManager {
    pub fn new(
        transaction_history_provider: impl TransactionHistoryProvider + 'static,
        customer_account_provider: impl CustomerAccountProvider + 'static,
    ) -> Self {
        DefaultTransactionsManager {
            transaction_history_provider: Box::new(transaction_history_provider),
            customer_account_provider: Box::new(customer_account_provider),
        }
    }

    fn is_duplicate_transaction_id(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<bool, String> {
        Ok(self
            .transaction_history_provider
            .as_mut()
            .read_transaction(transaction_id)?
            .is_some())
    }

    fn deposit(&mut self, transaction_request: TransactionRequest) -> Result<bool, String> {
        if self.is_duplicate_transaction_id(transaction_request.transaction_id)? {
            info!("Transaction with duplicate ID, skipping");
            return Ok(false);
        }
        let existing_amount = self
            .customer_account_provider
            .as_mut()
            .get_available(transaction_request.client_id)?
            .unwrap_or(Decimal::ZERO);
        self.customer_account_provider.as_mut().set_available(
            transaction_request.client_id,
            existing_amount
                + transaction_request
                    .amount
                    .expect("Transaction amount not present when depositing!"),
        )?;
        self.transaction_history_provider
            .as_mut()
            .write_transaction(transaction_request)?;
        Ok(true)
    }

    fn withdraw(&mut self, transaction_request: TransactionRequest) -> Result<bool, String> {
        if self.is_duplicate_transaction_id(transaction_request.transaction_id)? {
            info!("Transaction with duplicate ID, skipping");
            return Ok(false);
        }
        if let Some(locked) = self
            .customer_account_provider
            .as_mut()
            .get_locked_status(transaction_request.client_id)?
        {
            if locked {
                // We should not allow to withdraw money when the account is locked.
                // TODO Would be better if we could elaborate this with some errors.
                info!(
                    "The account of customer {} is locked, skipping withdrawal request.",
                    transaction_request.client_id
                );
                return Ok(false);
            }
        }
        // If the amount is not present, we just skip. Maybe we can add some logging later.
        if let Some(existing_amount) = self
            .customer_account_provider
            .as_mut()
            .get_available(transaction_request.client_id)?
        {
            let transaction_amount = transaction_request
                .amount
                .expect("Transaction amount not present when withdrawing!");
            if existing_amount >= transaction_amount {
                self.customer_account_provider.as_mut().set_available(
                    transaction_request.client_id,
                    existing_amount - transaction_amount,
                )?;
                self.transaction_history_provider
                    .write_transaction(transaction_request)?;
                return Ok(true);
            } else {
                info!(
                    "The customer {} doesn't have enough available funds to withdraw {}",
                    transaction_request.client_id, transaction_amount
                );
            }
        } else {
            info!(
                "The customer {} doens't have any available funds, skipping the withdraw request.",
                transaction_request.client_id
            );
        }
        Ok(false)
    }

    fn dispute(&mut self, transaction_request: TransactionRequest) -> Result<bool, String> {
        let existing_amount: Decimal = self
            .customer_account_provider
            .as_mut()
            .get_available(transaction_request.client_id)?
            .unwrap_or(Decimal::ZERO);
        if let Some(disputed_transaction) = self
            .transaction_history_provider
            .as_mut()
            .read_transaction(transaction_request.transaction_id)?
        {
            if disputed_transaction.client_id != transaction_request.client_id {
                info!("Client ID of the disputed transaction doesn't match the client ID of the request, possibly malicious client");
                return Ok(false);
            }

            let disputed_amount = disputed_transaction
                .amount
                .expect("Disputed transaction doesn't have amount");

            let disputed_transaction_state = self
                .transaction_history_provider
                .as_mut()
                .read_transaction_state(transaction_request.transaction_id)?;
            // Skipping if the transaction was already disputed or charged back
            if Some(true)
                == disputed_transaction_state.map(|state| state.held || state.charged_back)
            {
                info!(
                    "Transaction {} already on hold or charged back, not holding again",
                    transaction_request.transaction_id
                );
                return Ok(false);
            }
            // Allowing disputes even if they will create negative available funds. Customers first!

            // TODO: with ? failing at random moment, while this might break the consistency of the system. Think if some guarantee system can be implemented. Transactions?
            self.customer_account_provider.as_mut().set_available(
                transaction_request.client_id,
                existing_amount - disputed_amount,
            )?;
            let existing_held_amount = self
                .customer_account_provider
                .as_mut()
                .get_held_amount(transaction_request.client_id)?
                .unwrap_or(Decimal::ZERO);
            self.customer_account_provider.as_mut().set_held_amount(
                transaction_request.client_id,
                existing_held_amount + disputed_amount,
            )?;
            let new_transaction_state = disputed_transaction_state
                .map(|existing_state| {
                    let mut new_state = existing_state.clone();
                    new_state.held = true;
                    new_state
                })
                .unwrap_or_else(|| TransactionState {
                    held: true,
                    ..Default::default()
                });
            self.transaction_history_provider
                .as_mut()
                .write_transaction_state(
                    transaction_request.transaction_id,
                    new_transaction_state,
                )?;
            return Ok(true);
        }
        Ok(false)
    }

    fn resolve(&mut self, transaction_request: TransactionRequest) -> Result<bool, String> {
        let existing_amount = self
            .customer_account_provider
            .as_mut()
            .get_available(transaction_request.client_id)?
            .unwrap_or(Decimal::ZERO);
        if let Some(disputed_transaction) = self
            .transaction_history_provider
            .as_mut()
            .read_transaction(transaction_request.transaction_id)?
        {
            if disputed_transaction.client_id != transaction_request.client_id {
                info!("Client ID of the disputed transaction doesn't match the client ID of the request, possibly malicious client");
                return Ok(false);
            }

            let disputed_amount = disputed_transaction
                .amount
                .expect("Disputed transaction doesn't have amount");

            if let Some(disputed_transaction_state) = self
                .transaction_history_provider
                .as_mut()
                .read_transaction_state(transaction_request.transaction_id)?
            {
                // Skipping if the transaction was not held or was already charged_back
                if !disputed_transaction_state.held || disputed_transaction_state.charged_back {
                    info!(
                        "Transaction {} is not on hold or was charged back, not resolving",
                        transaction_request.transaction_id
                    );
                    return Ok(false);
                }
                if let Some(existing_held_amount) = self
                    .customer_account_provider
                    .as_mut()
                    .get_held_amount(transaction_request.client_id)?
                {
                    if existing_held_amount < disputed_amount {
                        panic!("Something went wrong, disputed transaction funds are not held");
                    }
                    self.customer_account_provider.as_mut().set_available(
                        transaction_request.client_id,
                        existing_amount + disputed_amount,
                    )?;
                    self.customer_account_provider.as_mut().set_held_amount(
                        transaction_request.client_id,
                        existing_held_amount - disputed_amount,
                    )?;
                    let mut new_transaction_state = disputed_transaction_state.clone();
                    new_transaction_state.held = false;
                    self.transaction_history_provider
                        .as_mut()
                        .write_transaction_state(
                            transaction_request.transaction_id,
                            new_transaction_state,
                        )?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn chargeback(&mut self, transaction_request: TransactionRequest) -> Result<bool, String> {
        if let Some(disputed_transaction) = self
            .transaction_history_provider
            .as_mut()
            .read_transaction(transaction_request.transaction_id)?
        {
            if disputed_transaction.client_id != transaction_request.client_id {
                info!("Client ID of the disputed transaction doesn't match the client ID of the request, possibly malicious client");
                return Ok(false);
            }

            let disputed_amount = disputed_transaction
                .amount
                .expect("Disputed transaction doesn't have amount");

            if let Some(disputed_transaction_state) = self
                .transaction_history_provider
                .as_mut()
                .read_transaction_state(transaction_request.transaction_id)?
            {
                // Skipping if the transaction was not held or was already charged_back
                if !disputed_transaction_state.held || disputed_transaction_state.charged_back {
                    info!(
                        "Transaction {} is not on hold or was charged back, not charging back",
                        transaction_request.transaction_id
                    );
                    return Ok(false);
                }
                if let Some(existing_held_amount) = self
                    .customer_account_provider
                    .as_mut()
                    .get_held_amount(transaction_request.client_id)?
                {
                    if existing_held_amount < disputed_amount {
                        panic!("Something went wrong, disputed transaction funds are not held");
                    }
                    self.customer_account_provider.as_mut().set_held_amount(
                        transaction_request.client_id,
                        existing_held_amount - disputed_amount,
                    )?;
                    self.customer_account_provider
                        .as_mut()
                        .set_locked_status(transaction_request.client_id, true)?;
                    let mut new_transaction_state = disputed_transaction_state.clone();
                    new_transaction_state.held = false;
                    new_transaction_state.charged_back = true;
                    self.transaction_history_provider
                        .as_mut()
                        .write_transaction_state(
                            transaction_request.transaction_id,
                            new_transaction_state,
                        )?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn has_positive_amount(transaction_request: &TransactionRequest) -> bool {
        transaction_request
            .amount
            .map_or(false, |amount| amount.gt(&Decimal::ZERO))
    }

    fn has_no_amount(transaction_request: &TransactionRequest) -> bool {
        transaction_request.amount.is_none()
    }
}

impl TransactionsManager for DefaultTransactionsManager {
    fn structure_validation(transaction_request: &TransactionRequest) -> bool {
        match &transaction_request.transaction_type {
            TransactionType::Deposit => Self::has_positive_amount(transaction_request),
            TransactionType::Withdrawal => Self::has_positive_amount(transaction_request),
            TransactionType::Dispute => Self::has_no_amount(transaction_request),
            TransactionType::Resolve => Self::has_no_amount(transaction_request),
            TransactionType::Chargeback => Self::has_no_amount(transaction_request),
        }
    }

    fn handle_transaction(
        &mut self,
        transaction_request: TransactionRequest,
    ) -> Result<bool, String> {
        // TODO think about the system consistency if something goes wrong
        // Maybe instead of thinking about current available amount, check the recent transactions and recalculate it? That will let us
        // fix the consistency issue.
        // Also maybe update the available and held funds at the same time instead of separate API calls?
        //
        match &transaction_request.transaction_type {
            TransactionType::Deposit => self.deposit(transaction_request),
            TransactionType::Withdrawal => self.withdraw(transaction_request),
            TransactionType::Dispute => self.dispute(transaction_request),
            TransactionType::Resolve => self.resolve(transaction_request),
            TransactionType::Chargeback => self.chargeback(transaction_request),
        }
    }

    fn print_report(&self) -> Result<(), String> {
        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .delimiter(b',')
            .from_writer(vec![]);
        if let Some(err) = self
            .customer_account_provider
            .list_accounts()?
            .iter()
            .inspect(|account| {
                if account.available.scale() > 4 || account.held.scale() > 4 {
                    panic!(
                        "Some available/held values have > 4 scale! {}, {}",
                        account.available, account.held
                    )
                }
            })
            .map(|account| writer.serialize(account))
            .filter(|e| e.is_err())
            .nth(0)
        {
            return err.map_err(|e| e.to_string());
        }
        println!(
            "{}",
            String::from_utf8(writer.into_inner().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        customer_account_provider::MockCustomerAccountProvider,
        transaction_history_provider::transaction_history_provider::MockTransactionHistoryProvider,
    };

    use super::*;
    #[test]
    fn deposit_works_as_expected_as_first_transaction() {
        let transaction_id = 1;
        let client_id = 1;
        let amount = Decimal::new(10, 4);
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        mock_history_provider
            .expect_write_transaction()
            .with(eq(transaction_request.clone()))
            .times(1)
            .return_const(Ok(()));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_available()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(None));
        mock_customer_account_provider
            .expect_set_available()
            .with(eq(client_id), eq(amount))
            .times(1)
            .return_const(Ok(()));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.deposit(transaction_request);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn deposit_works_as_expected_when_some_funds_already_present() {
        let transaction_id = 1;
        let client_id = 1;
        let amount = Decimal::new(10, 0);
        let existing_amount = Decimal::new(5, 0);
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        mock_history_provider
            .expect_write_transaction()
            .with(eq(transaction_request.clone()))
            .times(1)
            .return_const(Ok(()));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_available()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(Some(existing_amount)));
        mock_customer_account_provider
            .expect_set_available()
            .with(eq(client_id), eq(amount + existing_amount))
            .times(1)
            .return_const(Ok(()));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.deposit(transaction_request);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn withdraw_works_as_expected_in_happy_case() {
        let transaction_id = 1;
        let client_id = 1;
        let amount = Decimal::new(5, 0);
        let existing_amount = Decimal::new(10, 0);
        let locked = false;
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        mock_history_provider
            .expect_write_transaction()
            .with(eq(transaction_request.clone()))
            .times(1)
            .return_const(Ok(()));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_available()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(Some(existing_amount)));
        mock_customer_account_provider
            .expect_get_locked_status()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(Some(locked)));
        mock_customer_account_provider
            .expect_set_available()
            .with(eq(client_id), eq(existing_amount - amount))
            .times(1)
            .return_const(Ok(()));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.withdraw(transaction_request);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn withdraw_skips_when_no_enough_funds_present() {
        let transaction_id = 1;
        let client_id = 1;
        let amount = Decimal::new(10, 0);
        let existing_amount = Decimal::new(5, 0);
        let locked = false;
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_available()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(Some(existing_amount)));
        mock_customer_account_provider
            .expect_get_locked_status()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(Some(locked)));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.withdraw(transaction_request);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn withdraw_skips_when_no_account_state_found() {
        let transaction_id = 1;
        let client_id = 1;
        let amount = Decimal::new(5, 0);
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        let locked = false;
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_available()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(None));
        mock_customer_account_provider
            .expect_get_locked_status()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(Some(locked)));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.withdraw(transaction_request);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn withdraw_skips_when_account_locked() {
        let transaction_id = 1;
        let client_id = 1;
        let amount = Decimal::new(5, 0);
        let locked = true;
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Withdrawal,
            client_id,
            transaction_id,
            amount: Some(amount),
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_locked_status()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(Some(locked)));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.withdraw(transaction_request);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn dispute_does_nothing_when_transaction_not_found() {
        let transaction_id = 1;
        let client_id = 1;
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Dispute,
            client_id,
            transaction_id,
            amount: None,
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_available()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(None));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.dispute(transaction_request);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn resolve_does_nothing_when_transaction_not_found() {
        let transaction_id = 1;
        let client_id = 1;
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Resolve,
            client_id,
            transaction_id,
            amount: None,
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        let mut mock_customer_account_provider = MockCustomerAccountProvider::new();
        mock_customer_account_provider
            .expect_get_available()
            .with(eq(client_id))
            .times(1)
            .return_const(Ok(None));
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.resolve(transaction_request);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn chargeback_does_nothing_when_transaction_not_found() {
        let transaction_id = 1;
        let client_id = 1;
        let transaction_request = TransactionRequest {
            transaction_type: TransactionType::Chargeback,
            client_id,
            transaction_id,
            amount: None,
        };
        let mut mock_history_provider = MockTransactionHistoryProvider::new();
        mock_history_provider
            .expect_read_transaction()
            .with(eq(transaction_id))
            .times(1)
            .return_const(Ok(None));
        let mock_customer_account_provider = MockCustomerAccountProvider::new();
        let mut transactions_manager =
            DefaultTransactionsManager::new(mock_history_provider, mock_customer_account_provider);
        let result = transactions_manager.chargeback(transaction_request);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
