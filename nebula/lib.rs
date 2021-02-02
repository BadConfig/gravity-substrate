#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod nebula {
    use ink_storage::{
        collections::{hashmap::Entry, HashMap},
        traits::{PackedLayout, SpreadLayout},
    };
    use ibport::IBport;
    use gravity::Gravity;
    use std::mem::transmute;
    use web3::types::{Recovery,H160};
    use web3::signing::{ keccak256,recover};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InvalidRequest,
        NotFound,
        RoundAlreadyMutated,
        TokenError,
        ConculsReduce,
        SubscriberIdExists,
    }
    pub type Result<T> = core::result::Result<T, Error>;
   

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    struct Subscription {
        owner: AccountId,
        contact_address: AccountId,
        minimal_comformations: u64,
        reward: [u8;32],
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    struct Pulse {
        data_hash: [u8;32],
        height: [u8;32],
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    struct Oracle {
        owner: AccountId,
        is_online: bool,
        id_in_queue: [u8;32],
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    enum DataType {
        Int64,
        Str,
        Bytes,
    }

    #[ink(storage)]
    pub struct Nebula {
        gravity_contract: AccountId,
        benefitial_value: u128,
        oracles: Vec<[u8;32]>,
        data_type: DataType, 
        round_mutated: HashMap<[u8;32],bool>,
        oracles_queue: Vec<[u8;32]>,
        subscribers_queue: Vec<[u8;32]>,
        pulses_queue: Vec<[u8;32]>,
        subscriptions: HashMap<[u8;32],Subscription>,
    }

    impl Nebula {
        #[ink(constructor)]
        pub fn new(data_type: DataType, 
            gravity_contract: AccountId, 
            benefitial_value: u128,
            oracles: Vec<[u8;32]>
        ) -> Self {
            Self { 
                data_type: data_type,
                benefitial_value: benefitial_value,
                oracles: oracles,
                gravity_contract: gravity_contract,
                round_mutated: HashMap::new(),
                subscribers_queue: Vec::new(),
                oracles_queue: Vec::new(),
                pulses_queue: Vec::new(),
                subscriptions: HashMap::new(),
            }
        }

        #[ink(message)]
        pub fn hash_new_oracles(&self, new_oracles: Vec<[u8;32]>) -> [u8;32] {
            let mut new_oracles = new_oracles; 
            let mut data: Vec<u8> = Vec::new();
            for i in new_oracles.iter() {
                data.append(&mut Vec::from(&i[..]));
            }
            keccak256(&data[..])   
        }

        pub fn update_oracles(
            &mut self, 
            new_oracles: Vec<[u8;32]>, 
            v: Vec<u64>, 
            r: Vec<[u8;32]>, 
            s: Vec<[u8;32]>,
            round_id: [u8;32],
        ) -> Result<()> {
            if let Entry::Occupied(_) = self.round_mutated.entry(round_id) {
                return Err(Error::RoundAlreadyMutated);
            } 

            let hash = self.hash_new_oracles(new_oracles.clone());
            let gravity_instance = Gravity::from_account_id(self.gravity_contract);
            let consuls = gravity_instance.get_consuls();

            let mut count: u128 = 0;
            for i in 0..consuls.len() {
                let rec = Recovery::new(hash, v[i], r[i].into(), s[i].into())
                    .as_signature()
                    .unwrap();
                let addr = recover(&hash, &rec.0, rec.1).unwrap();
                let consul_addr = H160::from_slice(&consuls[i][0..20]);
                if addr == consul_addr { count += 1; }
            }

            if count < self.benefitial_value { return Err(Error::ConculsReduce); }
            self.oracles = new_oracles;
            self.round_mutated.insert(round_id,true);
            Ok(())
        }

        fn get_subscriber_id(&self, contact_address: AccountId, minimal_comformations: u64) -> [u8;32] {
            // parse all needed to byte arrays
            let msg_sig: [u8;4] = ink_env::decode_input().unwrap();
            let caller: [u8;32] = unsafe { transmute(self.env().caller()) };
            let contact_address: [u8;32] = unsafe { transmute(contact_address) };
            let minimal_comformations: [u8;4] = unsafe { transmute(minimal_comformations) };

            //concat into one byte array
            let data: Vec<u8> = Vec::new();
            data.extend_from_slice(&msg_sig[..]);
            data.extend_from_slice(&caller[..]);
            data.extend_from_slice(&contact_address[..]);
            data.extend_from_slice(&minimal_comformations[..]);
            
            keccak256(&data[..])
        }

        #[ink(message)]
        pub fn subscribe(
            &mut self,
            contact_address: AccountId,
            minimal_comformations: u64,
            reward: [u8;32],
        ) -> Result<()> {
            let new_sub_id = self.get_subscriber_id(contact_address,minimal_comformations); 
            match self.subscriptions.entry(new_sub_id) {
                Entry::Occupied(_) => Err(Error::SubscriberIdExists),
                Entry::Vacant(v) => {
                    v.insert(Subscription{
                        owner: self.env().caller(),
                        contact_address: contact_address,
                        minimal_comformations: minimal_comformations,
                        reward: reward,
                    });
                    Ok(())
                }
            }
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn default_works() {
        }
    }
}
