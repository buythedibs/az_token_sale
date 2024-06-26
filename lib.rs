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
        admin: AccountId,
        in_crypto: Option<AccountId>,
        out_token: AccountId,
        in_unit: Balance,
        out_unit: Balance,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AZTokenSale {
        admin: AccountId,
        in_crypto: Option<AccountId>,
        out_token: AccountId,
        in_unit: Balance,
        out_unit: Balance,
    }
    impl AZTokenSale {
        #[ink(constructor)]
        pub fn new(
            in_crypto: Option<AccountId>,
            out_token: AccountId,
            in_unit: Balance,
            out_unit: Balance,
        ) -> Self {
            Self {
                admin: Self::env().caller(),
                in_crypto,
                out_token,
                in_unit,
                out_unit,
            }
        }

        // === QUERIES ===
        #[ink(message)]
        pub fn config(&self) -> Config {
            Config {
                admin: self.admin,
                in_crypto: self.in_crypto,
                out_token: self.out_token,
                in_unit: self.in_unit,
                out_unit: self.out_unit,
            }
        }

        // === HANDLES ===
        #[ink(message)]
        pub fn add_amount_for_sale(&mut self, amount: Balance) -> Result<()> {
            let caller: AccountId = Self::env().caller();
            Self::authorise(self.admin, caller)?;
            // validate in amount is in units of in_unit
            if amount == 0 || amount % self.out_unit > 0 {
                return Err(AZTokenSaleError::UnprocessableEntity(
                    "Amount must be in multiples of out_unit".to_string(),
                ));
            }

            self.acquire_psp22(self.out_token, caller, amount)?;

            Ok(())
        }

        // === PRIVATE ===
        fn authorise(allowed: AccountId, received: AccountId) -> Result<()> {
            if allowed != received {
                return Err(AZTokenSaleError::Unauthorised);
            }

            Ok(())
        }

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
        const MOCK_IN_UNIT: Balance = 250;
        const MOCK_OUT_UNIT: Balance = 1;

        // === HELPERS ===
        fn init() -> (DefaultAccounts<DefaultEnvironment>, AZTokenSale) {
            let accounts = default_accounts();
            set_caller::<DefaultEnvironment>(accounts.alice);
            let token_sale = AZTokenSale::new(None, accounts.eve, MOCK_IN_UNIT, MOCK_OUT_UNIT);
            (accounts, token_sale)
        }

        // === TESTS ===
        // === TEST QUERIES ===
        #[ink::test]
        fn test_config() {
            let (accounts, token_sale) = init();
            let config = token_sale.config();
            // * it returns the config
            assert_eq!(config.admin, accounts.alice);
            assert_eq!(config.in_crypto, token_sale.in_crypto);
            assert_eq!(config.out_token, token_sale.out_token);
            assert_eq!(config.in_unit, token_sale.in_unit);
            assert_eq!(config.out_unit, token_sale.out_unit);
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
        const MOCK_IN_UNIT: Balance = 250;
        const MOCK_OUT_UNIT: Balance = 5;
        const TOKEN_BALANCE: Balance = 1_000_000_000_000_000_000;

        // === TYPES ===
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        // === HELPERS ===
        fn account_id(k: Keypair) -> AccountId {
            AccountId::try_from(k.public_key().to_account_id().as_ref())
                .expect("account keyring has a valid account id")
        }

        // === TEST HANDLES ===
        #[ink_e2e::test]
        async fn test_add_amount_for_sale(mut client: ::ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Instantiate to token
            let to_token_constructor = ButtonRef::new(
                TOKEN_BALANCE,
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
            let token_sale_constructor =
                AZTokenSaleRef::new(None, to_token_id, MOCK_IN_UNIT, MOCK_OUT_UNIT);
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

            let add_amount_for_sale_message = build_message::<AZTokenSaleRef>(token_sale_id)
                .call(|token_sale| token_sale.add_amount_for_sale(TOKEN_BALANCE));
            // when called by non-admin
            // * it raises an error
            let result = client
                .call_dry_run(&ink_e2e::charlie(), &add_amount_for_sale_message, 0, None)
                .await
                .return_value();
            assert_eq!(result, Err(AZTokenSaleError::Unauthorised));
            // when called by admin
            // = when amount added in is not divisible by out_unit
            let add_amount_for_sale_message = build_message::<AZTokenSaleRef>(token_sale_id)
                .call(|token_sale| token_sale.add_amount_for_sale(1));
            // # it raises an error
            let result = client
                .call_dry_run(&ink_e2e::alice(), &add_amount_for_sale_message, 0, None)
                .await
                .return_value();
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "Amount must be in multiples of out_unit".to_string()
                ))
            );
            // = when amount added in is divisible by out_unit
            // = * it transfers the token from admin to itself
            let increase_allowance_message = build_message::<ButtonRef>(to_token_id)
                .call(|to_token| to_token.increase_allowance(token_sale_id, u128::MAX));
            client
                .call(&ink_e2e::alice(), increase_allowance_message, 0, None)
                .await
                .unwrap();
            let add_amount_for_sale_message = build_message::<AZTokenSaleRef>(token_sale_id)
                .call(|token_sale| token_sale.add_amount_for_sale(MOCK_OUT_UNIT));
            client
                .call(&ink_e2e::alice(), add_amount_for_sale_message, 0, None)
                .await
                .unwrap();
            let balance_message = build_message::<ButtonRef>(to_token_id)
                .call(|button| button.balance_of(token_sale_id));
            let balance: Balance = client
                .call_dry_run(&ink_e2e::alice(), &balance_message, 0, None)
                .await
                .return_value();
            assert_eq!(MOCK_OUT_UNIT, balance);

            Ok(())
        }
    }
}
