#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;


#[ink::contract]
mod ibport {
    use ink_prelude::vec::Vec;
    use ink_env::{call::{FromAccountId, Selector, utils::ReturnType}, hash::Keccak256};
    use ink_storage::{
        collections::{hashmap::Entry, HashMap},
        traits::{PackedLayout, SpreadLayout},
    };
    use token::Token;
    use primitive_types::U256;

    //древняя джедайская техника
    use std::{convert::TryInto, mem::transmute};
    use std::convert::TryFrom;
    use web3::signing::keccak256;
    
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotNebula,
        InvalidRequest,
        NotFound,
        ErrorMintingTokens,
        InvalidRequestStatus,
        TokenError,
    }


    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    pub enum RequestStatus {
        None = 0,
        New = 1,
        Rejected = 2,
        Success = 3,
        Returned = 4,
    }
    impl TryFrom<u128> for RequestStatus {
        type Error = Error;
    
        fn try_from(item: u128) -> Result<Self> {
            use RequestStatus::*;
            match item {
                0 => Ok(RequestStatus::None),
                1 => Ok(New),
                2 => Ok(Rejected),
                3 => Ok(Success),
                4 => Ok(Returned),
                _ => Err(Error::InvalidRequestStatus),
            }
        }
    }
    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    struct UnwrapRequest {
        home_address: AccountId,
        foreign_address: AccountId,
        ammount: u128
    }

    /// Defines the storage of contract.
    /// 
    #[ink(storage)]
    pub struct IBport {
        nebula: AccountId,
        token: AccountId,
        swap_statuses: HashMap<u128, RequestStatus>,
        unwrap_requests: HashMap<u128, UnwrapRequest>,
    }

    impl IBport {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(nebula: AccountId, token: AccountId) -> Self {
            Self {
                nebula: nebula,
                token: token,
                swap_statuses: HashMap::new(),
                unwrap_requests: HashMap::new(),
            }
        }
        /// the incoming data lays as: [action(1),swapId(32),ammount(32),receiver_addr(32)] in a queue
        /// numbers here are *bytes* count
        /// TODO: cover by tests to verify serialization
        fn parse_mint(portion: &[u8; 32 + 32 + 32]) -> (u128, u128, AccountId) {
            let swap_id = U256::from(&portion[0..33]);
            let ammount = U256::from(&portion[33..65]);
            let receiver_addr =
                unsafe { transmute::<[u8; 32], AccountId>(portion[65..97].try_into().unwrap()) };
            (swap_id.as_u128(), ammount.as_u128(), receiver_addr)
        }

        fn mint(&mut self, swap_id: u128, ammount: u128, receiver_addr: AccountId) -> Result<()> {
            let mut token_instance = Token::from_account_id(self.token);
            let slot = match self.swap_statuses.entry(swap_id) {
                Entry::Vacant(v) => v,
                Entry::Occupied(_) => return Err(Error::InvalidRequestStatus),
            };
            token_instance
                .mint(ammount, receiver_addr)
                .map_err(| _ | Error::ErrorMintingTokens)?;
            slot.insert(RequestStatus::New);
            Ok(())
        }
        /// the incoming data lays as: [swapId(32),new_status32)] in a queue
        /// numbers here are *bytes* count
        fn parse_change(portion: &[u8; 32 + 32]) -> (u128, u128) {
            let swap_id = U256::from(&portion[0..33]);
            let new_status = U256::from(&portion[33..65]);
            (swap_id.as_u128(), new_status.as_u128())
        }

        fn change(&mut self, swap_id: u128, new_status: u128) -> Result<()> {
            let _ = match self.swap_statuses.get(&swap_id) {
                Some(RequestStatus::New) => (),
                _ => return Err(Error::InvalidRequestStatus),
            };
            let new_status: RequestStatus = new_status.try_into()?;
            self.swap_statuses.insert(swap_id, new_status);
            Ok(())
        }

        #[ink(message)]
        pub fn attach_value(&mut self, data_flow: Vec<u8>) -> Result<()> {
            if self.env().caller() != self.nebula { return Err(Error::NotNebula); }
            let mut i = 0;
            while i < data_flow.len() {
                let cmd: u8 = data_flow[i]; 
                i+=1;
                match cmd {
                    b'm' => {
                        let (swap_id, ammount, receiver_addr) = 
                            Self::parse_mint(&data_flow[i..i+96].try_into().unwrap());
                        i+= 32 * 3;
                        self.mint(swap_id, ammount, receiver_addr);
                    }
                    b'c' => {
                        let (swap_id, new_status) = Self::parse_change(&data_flow[i..i+64].try_into().unwrap());
                        i+=64;
                        self.change(swap_id, new_status);     
                    },
                    _ => return Err(Error::InvalidRequest),
                }
            }
            Ok(())
        }

        fn pack_and_encode(sender: AccountId, receiver: AccountId, block: BlockNumber, ammount: u128) -> [u8;32] {
            let mut res: Vec<u8> = Vec::new();
            let a: [u8;32] = unsafe { transmute(sender) };
            let mut a:Vec<u8> = a.into();
            res.append(&mut a);
            let a: [u8;32] = unsafe { transmute(receiver) };
            let mut a:Vec<u8> = a.into();
            res.append(&mut a);
            let a: [u8;8] = unsafe { transmute(block) };
            let mut a:Vec<u8> = a.into();
            res.append(&mut a);
            let a: [u8;32] = unsafe { transmute(sender) };
            let mut a:Vec<u8> = a.into();
            res.append(&mut a);
            let a: [u8;16] = unsafe { transmute(ammount) };
            let mut a:Vec<u8> = a.into();
            res.append(&mut a);
            keccak256(&res)
        }

        #[ink(message)]
        pub fn create_transfer_unwrap_request(&mut self, ammount: u128, receiver_addr: AccountId) -> Result<()> {
            let swap_id = Self::pack_and_encode(self.env().caller(), receiver_addr, self.env().block_number(), ammount);
            let mut token_instance: Token = Token::from_account_id(self.token);
            token_instance
                .burn_tokens(self.env().caller(), ammount)
                .map_err(|i| Error::TokenError)?;
            let swap_id = u128::from_be_bytes(swap_id[0..16].try_into().unwrap());
            self.unwrap_requests.insert(swap_id,UnwrapRequest{
                home_address: self.env().caller(),
                foreign_address: receiver_addr,
                ammount: ammount,
            });
            self.swap_statuses.insert(swap_id, RequestStatus::New);
            Ok(())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn size() {
            use std;
            println!("{}", std::mem::size_of::<[u8; 32]>());
        }
    }
}
