use sled::{Db, Tree};
use tempfile::NamedTempFile;

use crate::{
    common_types::TransactionId,
    transaction_request::{TransactionRequest, TransactionState},
};

use super::transaction_history_provider::TransactionHistoryProvider;

pub struct SledTransactionHistoryProvider {
    tree: Tree,
}

fn err_to_string(e: impl ToString) -> String {
    e.to_string()
}

impl SledTransactionHistoryProvider {
    pub fn new() -> Result<Self, String> {
        let file = NamedTempFile::new().map_err(err_to_string)?;
        let db = sled::open(file.path()).map_err(err_to_string)?;
        let tree = db.open_tree("a").map_err(err_to_string)?;
        Ok(SledTransactionHistoryProvider { tree })
    }
}
impl TransactionHistoryProvider for SledTransactionHistoryProvider {
    fn write_transaction<'a>(
        &'a mut self,
        transaction_request: TransactionRequest,
    ) -> Result<(), String> {
        // // Expensive operations, can be improved with zerocopy
        // let serialized: String = serde_json::to_string(&transaction_request).unwrap();
        // self.tree
        //     .insert(
        //         transaction_request.transaction_id.to_be_bytes(),
        //         serialized.as_bytes(),
        //     )
        //     .map_err(err_to_string)?;
        todo!()
    }

    fn read_transaction<'a>(
        &'a mut self,
        transaction_id: TransactionId,
    ) -> Result<Option<&'a TransactionRequest>, String> {
        // if let Some(val) = self
        //     .tree
        //     .get(transaction_id.to_be_bytes())
        //     .map_err(err_to_string)?
        // {
        //     return Ok(Some(
        //         serde_json::from_str(
        //             &String::from_utf8_lossy(val.as_ref()).map_err(err_to_string)?,
        //         )
        //         .map_err(err_to_string)?,
        //     ));
        // }
        todo!()
    }

    fn write_transaction_state<'a>(
        &'a mut self,
        transaction_id: TransactionId,
        transaction_state: TransactionState,
    ) -> Result<(), String> {
        todo!()
    }

    fn read_transaction_state<'a>(
        &'a mut self,
        transaction_id: TransactionId,
    ) -> Result<Option<&'a TransactionState>, String> {
        todo!()
    }
}
