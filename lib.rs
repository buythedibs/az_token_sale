#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
mod az_token_sale {
    use crate::errors::AZTokenSaleError;
    use ink::{env::CallFlags, prelude::string::ToString, prelude::vec};
    use openbrush::contracts::psp22::PSP22Ref;

    // === TYPES ===
    type Result<T> = core::result::Result<T, AZTokenSaleError>;

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        creator: AccountId,
        in_crypto: Option<AccountId>,
        out_token: AccountId,
        amount_for_sale: Balance,
        amount_available_for_sale: Balance,
        from_amount: Balance,
        to_amount: Balance,
        start_block: BlockNumber,
        end_block: BlockNumber,
        lock_up_release_block_frequency: Option<BlockNumber>,
        lock_up_release_percentage: Option<u8>,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AZTokenSale {
        creator: AccountId,
        in_crypto: Option<AccountId>,
        out_token: AccountId,
        amount_for_sale: Balance,
        amount_available_for_sale: Balance,
        from_amount: Balance,
        to_amount: Balance,
        start_block: BlockNumber,
        end_block: BlockNumber,
        lock_up_release_block_frequency: Option<BlockNumber>,
        lock_up_release_percentage: Option<u8>,
    }
    impl AZTokenSale {
        #[ink(constructor)]
        pub fn new(
            in_crypto: Option<AccountId>,
            out_token: AccountId,
            from_amount: Balance,
            to_amount: Balance,
            amount_for_sale: Balance,
            start_block: BlockNumber,
            end_block: BlockNumber,
            lock_up_release_block_frequency: Option<BlockNumber>,
            lock_up_release_percentage: Option<u8>,
        ) -> Self {
            Self {
                creator: Self::env().caller(),
                in_crypto,
                out_token,
                amount_for_sale,
                amount_available_for_sale: 0,
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
                amount_for_sale: self.amount_for_sale,
                amount_available_for_sale: self.amount_available_for_sale,
                from_amount: self.from_amount,
                to_amount: self.to_amount,
                start_block: self.start_block,
                end_block: self.end_block,
                lock_up_release_block_frequency: self.lock_up_release_block_frequency,
                lock_up_release_percentage: self.lock_up_release_percentage,
            }
        }

        // === HANDLES ===
        #[ink(message)]
        pub fn add_amount_for_sale(&mut self) -> Result<()> {
            if self.env().block_number() >= self.start_block {
                return Err(AZTokenSaleError::UnprocessableEntity(
                    "Token sale has already begun.".to_string(),
                ));
            }
            if self.amount_available_for_sale > 0 {
                return Err(AZTokenSaleError::UnprocessableEntity(
                    "Sale amount already added.".to_string(),
                ));
            }

            self.acquire_psp22(self.out_token, self.env().caller(), self.amount_for_sale)?;
            self.amount_available_for_sale = self.amount_for_sale;

            Ok(())
        }

        // === PRIVATE ===
        fn acquire_psp22(&self, token: AccountId, from: AccountId, amount: Balance) -> Result<()> {
            PSP22Ref::transfer_from_builder(&token, from, self.env().account_id(), amount, vec![])
                .call_flags(CallFlags::default())
                .invoke()?;

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{
            test::{default_accounts, set_caller, DefaultAccounts},
            DefaultEnvironment,
        };

        // === CONSTANTS ===
        const MOCK_FROM_AMOUNT: Balance = 250;
        const MOCK_TO_AMOUNT: Balance = 1;
        const MOCK_AMOUNT_FOR_SALE: Balance = 1_000_000_000_000_000_000;
        const MOCK_START_BLOCK: BlockNumber = 2;
        const MOCK_END_BLOCK: BlockNumber = 3;
        // Monthly
        const MOCK_LOCK_UP_RELEASE_BLOCK_FREQUENCY: BlockNumber = 60 * 60 * 24 * 30;
        const MOCK_LOCK_UP_RELEASE_PERCENTAGE: u8 = 25;

        // === HELPERS ===
        fn init() -> (DefaultAccounts<DefaultEnvironment>, AZTokenSale) {
            let accounts = default_accounts();
            set_caller::<DefaultEnvironment>(accounts.alice);
            let token_sale = AZTokenSale::new(
                None,
                accounts.eve,
                MOCK_FROM_AMOUNT,
                MOCK_TO_AMOUNT,
                MOCK_AMOUNT_FOR_SALE,
                MOCK_START_BLOCK,
                MOCK_END_BLOCK,
                Some(MOCK_LOCK_UP_RELEASE_BLOCK_FREQUENCY),
                Some(MOCK_LOCK_UP_RELEASE_PERCENTAGE),
            );
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
            assert_eq!(config.amount_for_sale, token_sale.amount_for_sale);
            assert_eq!(
                config.amount_available_for_sale,
                token_sale.amount_available_for_sale
            );
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

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use crate::az_token_sale::AZTokenSaleRef;
        use az_button::ButtonRef;
        use ink_e2e::{build_message, Keypair};
        use openbrush::contracts::traits::psp22::psp22_external::PSP22;

        // === CONSTANTS ===
        const MOCK_FROM_AMOUNT: Balance = 250;
        const MOCK_TO_AMOUNT: Balance = 1;
        const MOCK_AMOUNT_FOR_SALE: Balance = 1_000_000_000_000_000_000;
        const MOCK_START_BLOCK: BlockNumber = 2;
        const MOCK_END_BLOCK: BlockNumber = 3;
        // Monthly
        const MOCK_LOCK_UP_RELEASE_BLOCK_FREQUENCY: BlockNumber = 60 * 60 * 24 * 30;
        const MOCK_LOCK_UP_RELEASE_PERCENTAGE: u8 = 25;

        // === TYPES ===
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        // === HELPERS ===
        fn account_id(k: Keypair) -> AccountId {
            AccountId::try_from(k.public_key().to_account_id().as_ref())
                .expect("account keyring has a valid account id")
        }

        // === TEST HANDLES ===
        #[ink_e2e::test]
        async fn test_add_amount_for_sale_one(
            mut client: ::ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {
            // Instantiate to token
            let to_token_constructor = ButtonRef::new(
                MOCK_AMOUNT_FOR_SALE,
                Some("Button".to_string()),
                Some("BTN".to_string()),
                6,
            );
            let to_token_id: AccountId = client
                .instantiate(
                    "az_button",
                    &ink_e2e::alice(),
                    to_token_constructor,
                    0,
                    None,
                )
                .await
                .expect("Reward token instantiate failed")
                .account_id;
            let mut current_block = 0;
            // Instantiate token sale for smart contract
            let token_sale_constructor = AZTokenSaleRef::new(
                None,
                to_token_id,
                MOCK_FROM_AMOUNT,
                MOCK_TO_AMOUNT,
                MOCK_AMOUNT_FOR_SALE,
                MOCK_START_BLOCK,
                MOCK_END_BLOCK,
                Some(MOCK_LOCK_UP_RELEASE_BLOCK_FREQUENCY),
                Some(MOCK_LOCK_UP_RELEASE_PERCENTAGE),
            );
            let token_sale_id: AccountId = client
                .instantiate(
                    "az_token_sale",
                    &ink_e2e::alice(),
                    token_sale_constructor,
                    0,
                    None,
                )
                .await
                .expect("AZ Token Sale instantiate failed")
                .account_id;
            current_block += 1;
            // when current block is greater than or equal to start block
            // * it does not add amount for sale
            let add_amount_for_sale_message = build_message::<AZTokenSaleRef>(token_sale_id)
                .call(|token_sale| token_sale.add_amount_for_sale());
            let mut result = client
                .call_dry_run(&ink_e2e::alice(), &add_amount_for_sale_message, 0, None)
                .await
                .return_value();
            // = * it raises an error
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "Token sale has already begun.".to_string()
                ))
            );

            Ok(())
        }

        #[ink_e2e::test]
        async fn test_add_amount_for_sale_two(
            mut client: ::ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {
            // Instantiate to token
            let to_token_constructor = ButtonRef::new(
                MOCK_AMOUNT_FOR_SALE,
                Some("Button".to_string()),
                Some("BTN".to_string()),
                6,
            );
            let to_token_id: AccountId = client
                .instantiate(
                    "az_button",
                    &ink_e2e::alice(),
                    to_token_constructor,
                    0,
                    None,
                )
                .await
                .expect("Reward token instantiate failed")
                .account_id;
            let mut current_block = 0;
            // Instantiate token sale for smart contract
            let token_sale_constructor = AZTokenSaleRef::new(
                None,
                to_token_id,
                MOCK_FROM_AMOUNT,
                MOCK_TO_AMOUNT,
                MOCK_AMOUNT_FOR_SALE,
                MOCK_START_BLOCK + 100,
                MOCK_END_BLOCK,
                Some(MOCK_LOCK_UP_RELEASE_BLOCK_FREQUENCY),
                Some(MOCK_LOCK_UP_RELEASE_PERCENTAGE),
            );
            let token_sale_id: AccountId = client
                .instantiate(
                    "az_token_sale",
                    &ink_e2e::alice(),
                    token_sale_constructor,
                    0,
                    None,
                )
                .await
                .expect("AZ Token Sale instantiate failed")
                .account_id;
            current_block += 1;
            // Increase allowance for token sale
            let increase_allowance_message = build_message::<ButtonRef>(to_token_id)
                .call(|to_token| to_token.increase_allowance(token_sale_id, u128::MAX));
            client
                .call(&ink_e2e::alice(), increase_allowance_message, 0, None)
                .await
                .unwrap();
            current_block += 1;
            // when current block is less than start block
            let add_amount_for_sale_message = build_message::<AZTokenSaleRef>(token_sale_id)
                .call(|token_sale| token_sale.add_amount_for_sale());
            client
                .call(&ink_e2e::alice(), add_amount_for_sale_message, 0, None)
                .await
                .unwrap();
            // * it acquires the amount for sale
            let balance_message = build_message::<ButtonRef>(to_token_id)
                .call(|button| button.balance_of(token_sale_id));
            let balance: Balance = client
                .call_dry_run(&ink_e2e::alice(), &balance_message, 0, None)
                .await
                .return_value();
            assert_eq!(MOCK_AMOUNT_FOR_SALE, balance);
            // * it sets the amount available for sale to amount for sale
            let config_message = build_message::<AZTokenSaleRef>(token_sale_id)
                .call(|token_sale| token_sale.config());
            let config: Config = client
                .call_dry_run(&ink_e2e::alice(), &config_message, 0, None)
                .await
                .return_value();
            assert_eq!(MOCK_AMOUNT_FOR_SALE, config.amount_available_for_sale);
            // = when amount for sale has already beed added
            // = * it raises an error
            let add_amount_for_sale_message = build_message::<AZTokenSaleRef>(token_sale_id)
                .call(|token_sale| token_sale.add_amount_for_sale());
            let result = client
                .call_dry_run(&ink_e2e::alice(), &add_amount_for_sale_message, 0, None)
                .await
                .return_value();
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "Sale amount already added.".to_string()
                ))
            );

            Ok(())
        }
    }
}
