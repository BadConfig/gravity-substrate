#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod gravity {

    use ink_storage::{
        collections::{hashmap::Entry, HashMap},
        traits::{PackedLayout, SpreadLayout},
    };
    use web3::types::{Recovery,H160};
    use web3::signing::{ keccak256,recover};
    #[ink(storage)]
    pub struct Gravity {
        consuls_by_rounds: HashMap<[u8;32],Vec<[u8;32]>>,
        benefitial_value: u128,
        last_round: [u8;32],
    }

    impl Gravity {
        #[ink(constructor)]
        pub fn new(consuls: Vec<[u8;32]>, benefitial_value: u128) -> Self {
            let mut hm: HashMap<[u8;32],Vec<[u8;32]>> = HashMap::new();
            hm.insert([0u8;32], consuls);
            Self { 
                last_round: [0u8;32],
                consuls_by_rounds: hm,
                benefitial_value: benefitial_value,
            }
        }

        #[ink(message)]
        pub fn update_consuls(&mut self, new_consuls: Vec<[u8;32]>, v: Vec<u64>, r: Vec<[u8;32]>, s: Vec<[u8;32]>, round_id: [u8;32]) {
            if round_id < self.last_round { return; }
            let hash = self.hash_new_consuls(new_consuls.clone(),round_id);
            let mut count: u128 = 0;
            for i in 0..new_consuls.len() {
                let rec = Recovery::new(hash, v[i], r[i].into(), s[i].into())
                    .as_signature()
                    .unwrap();
                let addr = recover(&hash, &rec.0, rec.1).unwrap();
                let cons_addr = H160::from_slice(&new_consuls[i][0..20]);
                if addr == cons_addr { count += 1; }
            }
            if count < self.benefitial_value { return; }
            self.consuls_by_rounds.insert(round_id, new_consuls);
            self.last_round = round_id;
        }

        #[ink(message)]
        pub fn hash_new_consuls(&self, new_consuls: Vec<[u8;32]>, round_id: [u8;32]) -> [u8;32] {
            let mut new_consuls = new_consuls; 
            new_consuls.push(round_id);
            let mut data: Vec<u8> = Vec::new();
            for i in new_consuls.iter() {
                data.append(&mut Vec::from(&i[..]));
            }
            keccak256(&data[..])
        }

        #[ink(message)]
        pub fn get_consuls(&mut self) -> Vec<[u8;32]> {
            self.get_consuls_by_round_id(self.last_round)
        }
        
        #[ink(message)]
        pub fn get_consuls_by_round_id(&mut self, round_id: [u8;32]) -> Vec<[u8;32]> {
            self.consuls_by_rounds
                .get(&round_id)
                .unwrap_or(&Vec::new())
                .clone()
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
    }
}
