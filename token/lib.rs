#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;
pub use self::token::Token;

#[ink::contract]
mod token {
    use ink_storage::collections::{hashmap::Entry, HashMap, Vec};
    use ink_prelude::string::String;
    use std::convert::Into;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotOwner,
        NotEnoughMoney,
        NotFound,
    }
    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    pub struct Token {
        token_name: String,
        token_symbol: String,
        balances: HashMap<AccountId, u128>,
        total_supply: u128,
        deployers: Vec<AccountId>,
    }

    impl Token {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(name: String, symbol: String) -> Self {
            let mut deployers = Vec::new();
            deployers.push(Self::env().caller());
            Self {
                token_name: name,
                token_symbol: symbol,
                balances: HashMap::new(),
                deployers: deployers,
                total_supply: 0,
            }
        }

        fn is_deployer(&self, addr: AccountId) -> bool {
            self.deployers.iter().any(| &i | i == addr)
        }

        #[ink(message)]
        pub fn add_deployer(&mut self, adress: AccountId) -> Result<()> {
            if !self.is_deployer(self.env().caller()) {
                return Err(Error::NotOwner);
            }
            self.deployers.push(adress);
            Ok(())
        }

        #[ink(message)]
        pub fn mint(&mut self, ammount: u128, adress: AccountId) -> Result<()> {
            let sender = self.env().caller();

            if self.is_deployer(sender) {
                return Err(Error::NotOwner);
            }

            match self.balances.entry(adress) {
                Entry::Vacant(v) => {
                    v.insert(ammount.into());
                }
                Entry::Occupied(mut o) => {
                    let mut t = o.get().clone();
                    t += ammount;
                    o.insert(t);
                }
            }

            self.total_supply += ammount;
            Ok(())
        }

        #[ink(message)]
        pub fn balance_of(&self, account: AccountId) -> u128 {
            self
                .balances
                .get(&account)
                .unwrap_or(&0u128)
                .clone()
        }

        #[ink(message)]
        pub fn transfer_tokens(&mut self, to: AccountId, ammount: u128) -> Result<()> {
            self.transfer_from_to(self.env().caller(), to, ammount.into())
        }

        fn transfer_from_to(
            &mut self,
            from: AccountId,
            to: AccountId,
            ammount: u128,
        ) -> Result<()> {
            let sender_balance = self.balance_of(from);
            let to_balance = self.balance_of(to);

            if sender_balance < ammount {
                return Err(Error::NotEnoughMoney);
            }

            self
                .balances
                .insert(from, sender_balance - ammount);
            self
                .balances
                .insert(to, to_balance + ammount);
            Ok(())
        }

        #[ink(message)]
        pub fn burn_tokens(&mut self, account: AccountId, ammount: u128) -> Result<()> {
            let sender = self.env().caller();

            if self.is_deployer(sender) {
                return Err(Error::NotOwner);
            }

            let account_balance = self.balance_of(account);
            if account_balance < ammount {
                return Err(Error::NotEnoughMoney);
            }

            self.balances.insert(account, account_balance - ammount);

            self.total_supply -= ammount;
            Ok(())
        }
    }
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[test]
        fn default_works() {}
    }
}
