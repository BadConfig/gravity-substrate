#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod nebula {

    #[ink(storage)]
    pub struct Nebula {
        
    }

    impl Nebula {
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self { }
        }

        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn default_works() {
            let nebula = Nebula::default();
            assert_eq!(nebula.get(), false);
        }

        #[test]
        fn it_works() {
            let mut nebula = Nebula::new(false);
            assert_eq!(nebula.get(), false);
            nebula.flip();
            assert_eq!(nebula.get(), true);
        }
    }
}
