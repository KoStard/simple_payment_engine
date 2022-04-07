use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};

use crate::common_types::{CustomerId, TransactionId};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TransactionRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: CustomerId,
    #[serde(rename = "tx")]
    pub transaction_id: TransactionId,
    pub amount: Option<Decimal>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TransactionState {
    pub held: bool,
    pub charged_back: bool,
}