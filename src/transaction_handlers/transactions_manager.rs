use rust_decimal::Decimal;

use crate::{
    customer_account_provider::CustomerAccountProvider,
    transaction_history_provider::TransactionHistoryProvider,
    transaction_request::{TransactionRequest, TransactionState, TransactionType},
};

use super::transaction_handler::TransactionHandler;
use log::info;

pub struct TransactionsManager {
    // TODO Understand why Box
    // TODO remove the pubs
    pub transaction_history_provider: Box<dyn TransactionHistoryProvider>,
    pub customer_account_provider: Box<dyn CustomerAccountProvider>,
}
impl TransactionsManager {
    pub fn new(
        transaction_history_provider: impl TransactionHistoryProvider + 'static,
        customer_account_provider: impl CustomerAccountProvider + 'static,
    ) -> Self {
        TransactionsManager {
            transaction_history_provider: Box::new(transaction_history_provider),
            customer_account_provider: Box::new(customer_account_provider),
        }
    }
}

fn has_positive_amount(transaction_request: &TransactionRequest) -> bool {
    transaction_request
        .amount
        .map_or(false, |amount| amount.gt(&Decimal::ZERO))
}

fn has_no_amount(transaction_request: &TransactionRequest) -> bool {
    transaction_request.amount.is_none()
}

impl TransactionHandler for TransactionsManager {
    fn structure_validation(transaction_request: &TransactionRequest) -> bool {
        match &transaction_request.transaction_type {
            TransactionType::Deposit => has_positive_amount(transaction_request),
            TransactionType::Withdrawal => has_positive_amount(transaction_request),
            TransactionType::Dispute => has_no_amount(transaction_request),
            TransactionType::Resolve => has_no_amount(transaction_request),
            TransactionType::Chargeback => has_no_amount(transaction_request),
        }
    }

    fn handle_transaction(&mut self, transaction_request: TransactionRequest) -> Result<(), ()> {
        // TODO think about the system consistency if something goes wrong
        // Maybe instead of thinking about current available amount, check the recent transactions and recalculate it? That will let us
        // fix the consistency issue.
        // Also maybe update the available and held funds at the same time instead of separate API calls?
        //
        match &transaction_request.transaction_type {
            TransactionType::Deposit => {
                if self
                    .transaction_history_provider
                    .read_transaction(transaction_request.transaction_id)?
                    .is_some()
                {
                    info!("Transaction with duplicate ID, skipping");
                    return Ok(());
                }
                let existing_amount = self
                    .customer_account_provider
                    .get_available(transaction_request.client_id)?
                    .unwrap_or(Decimal::ZERO);
                self.customer_account_provider.set_available(
                    transaction_request.client_id,
                    existing_amount
                        + transaction_request
                            .amount
                            .expect("Transaction amount not present when depositing!"),
                )?;
                self.transaction_history_provider
                    .write_transaction(transaction_request)?;
            }
            TransactionType::Withdrawal => {
                if self
                    .transaction_history_provider
                    .read_transaction(transaction_request.transaction_id)?
                    .is_some()
                {
                    info!("Transaction with duplicate ID, skipping");
                    return Ok(());
                }
                if let Some(locked) = self
                    .customer_account_provider
                    .get_locked_status(transaction_request.client_id)?
                {
                    if locked {
                        // We should not allow to withdraw money when the account is locked.
                        // TODO Would be better if we could elaborate this with some errors.
                        info!(
                            "The account of customer {} is locked, skipping withdrawal request.",
                            transaction_request.client_id
                        );
                        return Ok(());
                    }
                }
                // If the amount is not present, we just skip. Maybe we can add some logging later.
                if let Some(existing_amount) = self
                    .customer_account_provider
                    .get_available(transaction_request.client_id)?
                {
                    let transaction_amount = transaction_request
                        .amount
                        .expect("Transaction amount not present when withdrawing!");
                    if existing_amount >= transaction_amount {
                        self.customer_account_provider.set_available(
                            transaction_request.client_id,
                            existing_amount - transaction_amount,
                        )?;
                        self.transaction_history_provider
                            .write_transaction(transaction_request)?;
                    } else {
                        info!(
                            "The customer {} doesn't have enough available funds to withdraw {}",
                            transaction_request.client_id, transaction_amount
                        );
                    }
                } else {
                    info!("The customer {} doens't have any available funds, skipping the withdraw request.", transaction_request.client_id);
                }
            }
            TransactionType::Dispute => {
                // If no amount exists, skipping
                // TODO: Possibly dangerous
                if let Some(existing_amount) = self
                    .customer_account_provider
                    .get_available(transaction_request.client_id)?
                {
                    if let Some(disputed_transaction) = self
                        .transaction_history_provider
                        .read_transaction(transaction_request.transaction_id)?
                    {
                        if disputed_transaction.client_id != transaction_request.client_id {
                            info!("Client ID of the disputed transaction doesn't match the client ID of the request, possibly malicious client");
                            return Ok(());
                        }

                        let disputed_amount = disputed_transaction
                            .amount
                            .expect("Disputed transaction doesn't have amount");

                        let disputed_transaction_state = self
                            .transaction_history_provider
                            .read_transaction_state(transaction_request.transaction_id)?;
                        // Skipping if the transaction was already disputed or charged back
                        if Some(true)
                            == disputed_transaction_state
                                .map(|state| state.held || state.charged_back)
                        {
                            info!(
                                "Transaction {} already on hold or charged back, not holding again",
                                transaction_request.transaction_id
                            );
                            return Ok(());
                        }
                        // Allowing disputes even if they will create negative available funds. Customers first!

                        // TODO: with ? failing at random moment, while this might break the consistency of the system. Think if some guarantee system can be implemented. Transactions?
                        self.customer_account_provider.set_available(
                            transaction_request.client_id,
                            existing_amount - disputed_amount,
                        )?;
                        let existing_held_amount = self
                            .customer_account_provider
                            .get_held_amount(transaction_request.client_id)?
                            .unwrap_or(Decimal::ZERO);
                        self.customer_account_provider.set_held_amount(
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
                        self.transaction_history_provider.write_transaction_state(
                            transaction_request.transaction_id,
                            new_transaction_state,
                        )?;
                    }
                }
            }
            TransactionType::Resolve => {
                // TODO implement mechanism for preventing the transactions from getting disputed/resolved/charged back multiple times!
                let existing_amount = self
                    .customer_account_provider
                    .get_available(transaction_request.client_id)?
                    .unwrap_or(Decimal::ZERO);
                if let Some(disputed_transaction) = self
                    .transaction_history_provider
                    .read_transaction(transaction_request.transaction_id)?
                {
                    if disputed_transaction.client_id != transaction_request.client_id {
                        info!("Client ID of the disputed transaction doesn't match the client ID of the request, possibly malicious client");
                        return Ok(());
                    }

                    let disputed_amount = disputed_transaction
                        .amount
                        .expect("Disputed transaction doesn't have amount");

                    if let Some(disputed_transaction_state) = self
                        .transaction_history_provider
                        .read_transaction_state(transaction_request.transaction_id)?
                    {
                        // Skipping if the transaction was not held or was already charged_back
                        if !disputed_transaction_state.held
                            || disputed_transaction_state.charged_back
                        {
                            info!(
                                "Transaction {} is not on hold or was charged back, not resolving",
                                transaction_request.transaction_id
                            );
                            return Ok(());
                        }
                        if let Some(existing_held_amount) = self
                            .customer_account_provider
                            .get_held_amount(transaction_request.client_id)?
                        {
                            if existing_held_amount < disputed_amount {
                                panic!(
                                    "Something went wrong, disputed transaction funds are not held"
                                );
                            }
                            self.customer_account_provider.set_available(
                                transaction_request.client_id,
                                existing_amount + disputed_amount,
                            )?;
                            self.customer_account_provider.set_held_amount(
                                transaction_request.client_id,
                                existing_held_amount - disputed_amount,
                            )?;
                            let mut new_transaction_state = disputed_transaction_state.clone();
                            new_transaction_state.held = false;
                            self.transaction_history_provider.write_transaction_state(
                                transaction_request.transaction_id,
                                new_transaction_state,
                            )?;
                        }
                    }
                }
            }
            // TODO VERIFY THAT THE CUSTOMER ID AND THE ORIGINAL TRANSACTION CUSTOMER ID ARE THE SAME
            TransactionType::Chargeback => {
                if let Some(disputed_transaction) = self
                    .transaction_history_provider
                    .read_transaction(transaction_request.transaction_id)?
                {
                    if disputed_transaction.client_id != transaction_request.client_id {
                        info!("Client ID of the disputed transaction doesn't match the client ID of the request, possibly malicious client");
                        return Ok(());
                    }

                    let disputed_amount = disputed_transaction
                        .amount
                        .expect("Disputed transaction doesn't have amount");

                    if let Some(disputed_transaction_state) = self
                        .transaction_history_provider
                        .read_transaction_state(transaction_request.transaction_id)?
                    {
                        // Skipping if the transaction was not held or was already charged_back
                        if !disputed_transaction_state.held
                            || disputed_transaction_state.charged_back
                        {
                            info!(
                                "Transaction {} is not on hold or was charged back, not charging back",
                                transaction_request.transaction_id
                            );
                            return Ok(());
                        }
                        if let Some(existing_held_amount) = self
                            .customer_account_provider
                            .get_held_amount(transaction_request.client_id)?
                        {
                            if existing_held_amount < disputed_amount {
                                panic!(
                                    "Something went wrong, disputed transaction funds are not held"
                                );
                            }
                            self.customer_account_provider.set_held_amount(
                                transaction_request.client_id,
                                existing_held_amount - disputed_amount,
                            )?;
                            self.customer_account_provider
                                .set_locked_status(transaction_request.client_id, true)?;
                            let mut new_transaction_state = disputed_transaction_state.clone();
                            new_transaction_state.held = false;
                            new_transaction_state.charged_back = true;
                            self.transaction_history_provider.write_transaction_state(
                                transaction_request.transaction_id,
                                new_transaction_state,
                            )?;
                        }
                    }
                }
            }
        };
        Ok(())
    }
}
