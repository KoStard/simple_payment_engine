use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::common_types::CustomerId;

// TODO if we lock this instance, the performance will not be good. Instead we can lock per customer.
pub trait CustomerAccountProvider {
    fn get_available(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, ()>;
    fn get_held_amount(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, ()>;
    fn get_locked_status(&mut self, customer_id: CustomerId) -> Result<Option<bool>, ()>;
    fn set_available(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), ()>;
    fn set_held_amount(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), ()>;
    fn set_locked_status(&mut self, customer_id: CustomerId, locked: bool) -> Result<(), ()>;
}

// Not exposed externally
struct CustomerAccount {
    available: Decimal,
    held: Decimal,
    locked: bool,
}

impl CustomerAccount {
    fn new(available: Decimal, held: Decimal, locked: bool) -> Self {
        CustomerAccount { available, held, locked }
    }
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
    fn get_available(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, ()> {
        Ok(self.storage.get(&customer_id).map(|c| c.available))
    }

    fn get_held_amount(&mut self, customer_id: CustomerId) -> Result<Option<Decimal>, ()> {
        Ok(self.storage.get(&customer_id).map(|c| c.held))
    }

    fn get_locked_status(&mut self, customer_id: CustomerId) -> Result<Option<bool>, ()> {
        Ok(self.storage.get(&customer_id).map(|c| c.locked))
    }

    fn set_available(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), ()> {
        if let Some(customer_account) = self.storage.get_mut(&customer_id) {
            customer_account.available = balance;
        } else {
            self.storage.insert(customer_id, CustomerAccount::new(balance, Decimal::ZERO, false));
        }
        Ok(())
    }

    fn set_held_amount(&mut self, customer_id: CustomerId, balance: Decimal) -> Result<(), ()> {
        if let Some(customer_account) = self.storage.get_mut(&customer_id) {
            customer_account.held = balance;
        } else {
            // TODO Think about this!
            panic!("Putting amount on hold on a non-existing account");
        }
        Ok(())
    }

    fn set_locked_status(&mut self, customer_id: CustomerId, locked: bool) -> Result<(), ()> {
        if let Some(customer_account) = self.storage.get_mut(&customer_id) {
            customer_account.locked = locked;
        } else {
            // TODO Think about this!
            panic!("Locking a non-existing account");
        }
        Ok(())
    }
}
