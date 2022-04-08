use mockall::predicate::*;
use mockall::*;

use crate::{
    common_types::TransactionId,
    transaction_request::{TransactionRequest, TransactionState},
};

/**
 * This trait is supposed to abstract all history providers and many of them will contain network calls or storage reads.
 * So we can expect that in some cases this will include failures that are not related to the transaction/state existance or consistency. 
 * Hence we need to allow the future instances to use these Results. We can also add different types of Errors.
 */
#[automock]
pub trait TransactionHistoryProvider {
    fn write_transaction<'a>(
        &'a mut self,
        transaction_request: TransactionRequest,
    ) -> Result<(), ()>;
    fn read_transaction<'a>(
        &'a mut self,
        transaction_id: TransactionId,
    ) -> Result<Option<&'a TransactionRequest>, ()>;
    fn write_transaction_state<'a>(
        &'a mut self,
        transaction_id: TransactionId,
        transaction_state: TransactionState,
    ) -> Result<(), ()>;
    fn read_transaction_state<'a>(
        &'a mut self,
        transaction_id: TransactionId,
    ) -> Result<Option<&'a TransactionState>, ()>;
}

