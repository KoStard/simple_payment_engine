use std::collections::HashMap;

use mockall::predicate::*;
use mockall::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::common_types::CustomerId;

#[automock]
pub trait CustomerAccountProvider {
    fn get_available(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, String>;
    fn get_held_amount(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, String>;
    fn get_locked_status(&mut self, customer_id: CustomerId) -> Result<Option<bool>, String>;
    fn set_available(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), String>;
    fn set_held_amount(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), String>;
    fn set_locked_status(&mut self, customer_id: CustomerId, locked: bool) -> Result<(), String>;
    fn list_accounts(&self) -> Result<Vec<CustomerAccountReport>, String>;
}

#[derive(Default)]
struct CustomerAccount {
    available: Decimal,
    held: Decimal,
    locked: bool,
}

impl CustomerAccount {
    fn new(available: Decimal, held: Decimal, locked: bool) -> Self {
        CustomerAccount {
            available,
            held,
            locked,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerAccountReport {
    pub client: CustomerId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

pub struct InMemoryCustomerAccountProvider {
    storage: HashMap<CustomerId, CustomerAccount>,
}

impl InMemoryCustomerAccountProvider {
    pub fn new() -> Self {
        InMemoryCustomerAccountProvider {
            storage: HashMap::new(),
        }
    }
}
impl CustomerAccountProvider for InMemoryCustomerAccountProvider {
    fn get_available(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, String> {
        Ok(self.storage.get(&customer_id).map(|c| c.available))
    }

    fn get_held_amount(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, String> {
        Ok(self.storage.get(&customer_id).map(|c| c.held))
    }

    fn get_locked_status(&mut self, customer_id: CustomerId) -> Result<Option<bool>, String> {
        Ok(self.storage.get(&customer_id).map(|c| c.locked))
    }

    fn set_available(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), String> {
        if let Some(customer_account) = self.storage.get_mut(&customer_id) {
            customer_account.available = balance;
        } else {
            self.storage.insert(
                customer_id,
                CustomerAccount::new(balance, Decimal::ZERO, false),
            );
        }
        Ok(())
    }

    fn set_held_amount(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), String> {
        if let Some(customer_account) = self.storage.get_mut(&customer_id) {
            customer_account.held = balance;
        } else {
            // Something went wrong... If the original transaction existed, then the account would exist as well
            panic!("Putting amount on hold on a non-existing account");
        }
        Ok(())
    }

    fn set_locked_status(&mut self, customer_id: CustomerId, locked: bool) -> Result<(), String> {
        if let Some(customer_account) = self.storage.get_mut(&customer_id) {
            customer_account.locked = locked;
        } else {
            // Something went wrong... If the original transaction existed, then the account would exist as well
            panic!("Locking a non-existing account");
        }
        Ok(())
    }

    fn list_accounts(&self) -> Result<Vec<CustomerAccountReport>, String> {
        return Ok(self
            .storage
            .iter()
            .map(|(client, account)| CustomerAccountReport {
                client: *client,
                available: account.available,
                held: account.held,
                locked: account.locked,
                total: account.available + account.held,
            })
            .collect());
    }
}

#[cfg(test)]
mod in_memory_customer_account_provider_tests {
    use super::*;

    #[test]
    fn get_available_works_as_expected_with_existing_account() {
        let customer_id = 1;
        let available = Decimal::new(10, 0);
        let mut storage = HashMap::new();
        storage.insert(
            customer_id,
            CustomerAccount {
                available,
                ..Default::default()
            },
        );
        let mut customer_account_provider = InMemoryCustomerAccountProvider { storage };
        assert_eq!(
            customer_account_provider.get_available(customer_id),
            Ok(Some(available))
        );
    }

    #[test]
    fn get_available_works_as_expected_with_missing_account() {
        let customer_id = 1;
        let storage = HashMap::new();
        let mut customer_account_provider = InMemoryCustomerAccountProvider { storage };
        assert_eq!(
            customer_account_provider.get_available(customer_id),
            Ok(None)
        );
    }

    #[test]
    fn get_held_amount_works_as_expected_with_existing_account() {
        let customer_id = 1;
        let held = Decimal::new(10, 0);
        let mut storage = HashMap::new();
        storage.insert(
            customer_id,
            CustomerAccount {
                held,
                ..Default::default()
            },
        );
        let mut customer_account_provider = InMemoryCustomerAccountProvider { storage };
        assert_eq!(
            customer_account_provider.get_held_amount(customer_id),
            Ok(Some(held))
        );
    }

    #[test]
    fn get_held_amount_works_as_expected_with_missing_account() {
        let customer_id = 1;
        let storage = HashMap::new();
        let mut customer_account_provider = InMemoryCustomerAccountProvider { storage };
        assert_eq!(
            customer_account_provider.get_held_amount(customer_id),
            Ok(None)
        );
    }

    #[test]
    fn get_locked_status_works_as_expected_with_existing_account() {
        let customer_id = 1;
        let locked = false;
        let mut storage = HashMap::new();
        storage.insert(
            customer_id,
            CustomerAccount {
                locked,
                ..Default::default()
            },
        );
        let mut customer_account_provider = InMemoryCustomerAccountProvider { storage };
        assert_eq!(
            customer_account_provider.get_locked_status(customer_id),
            Ok(Some(locked))
        );
    }

    #[test]
    fn get_locked_status_works_as_expected_with_missing_account() {
        let customer_id = 1;
        let storage = HashMap::new();
        let mut customer_account_provider = InMemoryCustomerAccountProvider { storage };
        assert_eq!(
            customer_account_provider.get_locked_status(customer_id),
            Ok(None)
        );
    }

    #[test]
    fn set_available_works_as_expected() {
        let customer_id = 1;
        let balance = Decimal::new(10, 0);
        let mut customer_account_provider = InMemoryCustomerAccountProvider::new();
        assert!(customer_account_provider
            .set_available(customer_id, balance)
            .is_ok());
        assert_eq!(
            customer_account_provider.get_available(customer_id),
            Ok(Some(balance))
        );
    }

    #[test]
    fn set_held_amount_works_as_expected() {
        let customer_id = 1;
        let balance = Decimal::new(10, 0);
        let mut customer_account_provider = InMemoryCustomerAccountProvider::new();
        customer_account_provider
            .set_available(customer_id, Decimal::new(10, 0))
            .expect("Couldn't create the account");
        assert!(customer_account_provider
            .set_held_amount(customer_id, balance)
            .is_ok());
        assert_eq!(
            customer_account_provider.get_held_amount(customer_id),
            Ok(Some(balance))
        );
    }

    #[test]
    #[should_panic]
    fn set_held_amount_panics_when_no_account_found() {
        let customer_id = 1;
        let balance = Decimal::new(10, 0);
        let mut customer_account_provider = InMemoryCustomerAccountProvider::new();
        let _ = customer_account_provider.set_held_amount(customer_id, balance);
    }

    #[test]
    fn set_locked_status_works_as_expected() {
        let customer_id = 1;
        let locked = true;
        let mut customer_account_provider = InMemoryCustomerAccountProvider::new();
        customer_account_provider
            .set_available(customer_id, Decimal::new(10, 0))
            .expect("Couldn't create the account");
        assert!(customer_account_provider
            .set_locked_status(customer_id, locked)
            .is_ok());
        assert_eq!(
            customer_account_provider.get_locked_status(customer_id),
            Ok(Some(locked))
        );
    }

    #[test]
    #[should_panic]
    fn set_locked_status_panics_when_no_account_found() {
        let customer_id = 1;
        let locked = true;
        let mut customer_account_provider = InMemoryCustomerAccountProvider::new();
        let _ = customer_account_provider.set_locked_status(customer_id, locked);
    }

    #[test]
    fn list_accounts_works_as_expected() {
        let mut customer_account_provider = InMemoryCustomerAccountProvider::new();
        customer_account_provider
            .set_available(1, Decimal::new(10, 0))
            .unwrap();
        customer_account_provider
            .set_available(2, Decimal::new(11, 0))
            .unwrap();
        customer_account_provider
            .set_held_amount(2, Decimal::new(12, 0))
            .unwrap();
        let accounts = customer_account_provider.list_accounts();
        let expected_accounts = vec![
            CustomerAccountReport {
                client: 1,
                available: Decimal::new(10, 0),
                held: Decimal::new(0, 0),
                total: Decimal::new(10, 0),
                locked: false,
            },
            CustomerAccountReport {
                client: 2,
                available: Decimal::new(11, 0),
                held: Decimal::new(12, 0),
                total: Decimal::new(23, 0),
                locked: false,
            },
        ];
        assert!(accounts.is_ok());
        let accounts = accounts.unwrap();
        assert!(expected_accounts.len() == accounts.len());
        assert!(expected_accounts
            .iter()
            .all(|account| accounts.contains(account)));
    }
}
