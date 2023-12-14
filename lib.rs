#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod az_token_sale {
    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        creator: AccountId,
        in_crypto: Option<AccountId>,
        out_token: AccountId,
        from_amount: Balance,
        to_amount: Balance,
        start_block: BlockNumber,
        end_block: BlockNumber,
        lock_up_release_block_frequency: Option<BlockNumber>,
        lock_up_release_percentage: Option<BlockNumber>,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AZTokenSale {
        creator: AccountId,
        in_crypto: Option<AccountId>,
        out_token: AccountId,
        from_amount: Balance,
        to_amount: Balance,
        start_block: BlockNumber,
        end_block: BlockNumber,
        lock_up_release_block_frequency: Option<BlockNumber>,
        lock_up_release_percentage: Option<BlockNumber>,
    }
    impl AZTokenSale {
        #[ink(constructor)]
        pub fn new(
            in_crypto: Option<AccountId>,
            out_token: AccountId,
            from_amount: Balance,
            to_amount: Balance,
            start_block: BlockNumber,
            end_block: BlockNumber,
            lock_up_release_block_frequency: Option<BlockNumber>,
            lock_up_release_percentage: Option<BlockNumber>,
        ) -> Self {
            Self {
                creator: Self::env().caller(),
                in_crypto,
                out_token,
                from_amount,
                to_amount,
                start_block,
                end_block,
                lock_up_release_block_frequency,
                lock_up_release_percentage,
            }
        }

        // === QUERIES ===
        #[ink(message)]
        pub fn config(&self) -> Config {
            Config {
                creator: self.creator,
                in_crypto: self.in_crypto,
                out_token: self.out_token,
                from_amount: self.from_amount,
                to_amount: self.to_amount,
                start_block: self.start_block,
                end_block: self.end_block,
                lock_up_release_block_frequency: self.lock_up_release_block_frequency,
                lock_up_release_percentage: self.lock_up_release_percentage,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{
            test::{default_accounts, set_caller, DefaultAccounts},
            DefaultEnvironment,
        };

        // === HELPERS ===
        fn init() -> (DefaultAccounts<DefaultEnvironment>, AZTokenSale) {
            let accounts = default_accounts();
            set_caller::<DefaultEnvironment>(accounts.alice);
            let token_sale = AZTokenSale::new(None, accounts.eve, 5, 1, 2, 2, Some(1), Some(25));
            (accounts, token_sale)
        }

        // === TESTS ===
        // === TEST QUERIES ===
        #[ink::test]
        fn test_config() {
            let (accounts, token_sale) = init();
            let config = token_sale.config();
            // * it returns the config
            assert_eq!(config.creator, accounts.alice);
            assert_eq!(config.in_crypto, token_sale.in_crypto);
            assert_eq!(config.out_token, token_sale.out_token);
            assert_eq!(config.from_amount, token_sale.from_amount);
            assert_eq!(config.to_amount, token_sale.to_amount);
            assert_eq!(config.start_block, token_sale.start_block);
            assert_eq!(config.end_block, token_sale.end_block);
            assert_eq!(
                config.lock_up_release_block_frequency,
                token_sale.lock_up_release_block_frequency
            );
            assert_eq!(
                config.lock_up_release_percentage,
                token_sale.lock_up_release_percentage
            );
        }
    }
}
