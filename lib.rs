#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
mod az_token_sale {
    use crate::errors::AZTokenSaleError;
    use ink::{env::CallFlags, prelude::string::ToString, prelude::vec};
    use openbrush::contracts::psp22::PSP22Ref;
    use primitive_types::U256;

    // === TYPES ===
    type Result<T> = core::result::Result<T, AZTokenSaleError>;

    // === STRUCTS ===
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Config {
        admin: AccountId,
        out_token: AccountId,
        in_unit: Balance,
        out_unit: Balance,
    }

    // === CONTRACT ===
    #[ink(storage)]
    pub struct AZTokenSale {
        admin: AccountId,
        out_token: AccountId,
        in_unit: Balance,
        out_unit: Balance,
    }
    impl AZTokenSale {
        #[ink(constructor)]
        pub fn new(out_token: AccountId, in_unit: Balance, out_unit: Balance) -> Self {
            Self {
                admin: Self::env().caller(),
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

        #[ink(message, payable)]
        pub fn buy(&mut self) -> Result<(Balance, Balance)> {
            let caller: AccountId = Self::env().caller();
            // validate in amount is in units of in_unit
            let mut in_amount: Balance = self.env().transferred_value();
            if in_amount == 0 || in_amount % self.in_unit > 0 {
                return Err(AZTokenSaleError::UnprocessableEntity(
                    "In amount must be in multiples of in_unit".to_string(),
                ));
            }
            // validate balance is positive
            let contract_address: AccountId = Self::env().account_id();
            let contract_balance: Balance = PSP22Ref::balance_of(&self.out_token, contract_address);
            if contract_balance == 0 {
                return Err(AZTokenSaleError::UnprocessableEntity(
                    "Sold out".to_string(),
                ));
            }

            // Calculate max in amount for refund
            let desired_out_amount: Balance = in_amount * self.out_unit / self.in_unit;
            let max_in_amount: Balance = if contract_balance >= desired_out_amount {
                in_amount
            } else {
                (U256::from(in_amount) * U256::from(contract_balance)
                    / U256::from(desired_out_amount))
                .as_u128()
            };

            // refund if necessary
            if in_amount > max_in_amount {
                let refund_amount: Balance = in_amount - max_in_amount;
                self.transfer_azero(caller, refund_amount)?;
                in_amount = max_in_amount
            }

            // Trasfer out token to user
            let out_amount: Balance = (U256::from(in_amount) * U256::from(self.out_unit)
                / U256::from(self.in_unit))
            .as_u128();
            PSP22Ref::transfer_builder(&self.out_token, caller, out_amount, vec![])
                .call_flags(CallFlags::default())
                .invoke()?;

            // Send AZERO to admin
            self.transfer_azero(self.admin, in_amount)?;

            Ok((in_amount, out_amount))
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

        fn transfer_azero(&self, address: AccountId, amount: Balance) -> Result<()> {
            if self.env().transfer(address, amount).is_err() {
                return Err(AZTokenSaleError::UnprocessableEntity(
                    "Insufficient AZERO balance".to_string(),
                ));
            }

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
            let token_sale = AZTokenSale::new(accounts.eve, MOCK_IN_UNIT, MOCK_OUT_UNIT);
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
            assert_eq!(config.out_token, token_sale.out_token);
            assert_eq!(config.in_unit, token_sale.in_unit);
            assert_eq!(config.out_unit, token_sale.out_unit);
        }

        #[ink::test]
        fn test_buy() {
            let (_accounts, mut az_token_sale) = init();

            // when in amount is zero
            // * it raises an error
            let mut result = az_token_sale.buy();
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "In amount must be in multiples of in_unit".to_string()
                ))
            );
            // when in amount is positive
            // = when in amount is not a multiple of in_unit
            // = * it raises an error
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(MOCK_IN_UNIT + 1);
            result = az_token_sale.buy();
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "In amount must be in multiples of in_unit".to_string(),
                )),
            );
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(MOCK_IN_UNIT - 1);
            result = az_token_sale.buy();
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "In amount must be in multiples of in_unit".to_string()
                ))
            );
            // = when in amount is a multiple of in_unit
            // REST WILL HAVE TO GO INTO INTEGRATION TEST AS IT CALLS AIRDROP SMART CONTRACT
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use crate::az_token_sale::AZTokenSaleRef;
        use az_button::ButtonRef;
        use ink_e2e::build_message;
        use ink_e2e::Keypair;
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
            // Instantiate token sale for smart contract
            let token_sale_constructor =
                AZTokenSaleRef::new(to_token_id, MOCK_IN_UNIT, MOCK_OUT_UNIT);
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

        #[ink_e2e::test]
        async fn test_buy(mut client: ::ink_e2e::Client<C, E>) -> E2EResult<()> {
            let alice_account_id: AccountId = account_id(ink_e2e::alice());
            let bob_account_id: AccountId = account_id(ink_e2e::bob());

            // Instantiate token
            let token_constructor = ButtonRef::new(
                TOKEN_BALANCE,
                Some("DIBS".to_string()),
                Some("DIBS".to_string()),
                12,
            );
            let to_token_id: AccountId = client
                .instantiate("az_button", &ink_e2e::alice(), token_constructor, 0, None)
                .await
                .expect("Token instantiate failed")
                .account_id;

            // Instantiate token sale for smart contract
            let token_sale_constructor =
                AZTokenSaleRef::new(to_token_id, MOCK_IN_UNIT, MOCK_OUT_UNIT);
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

            // when in amount is zero
            // * it raises an error
            let buy_message =
                build_message::<AZTokenSaleRef>(token_sale_id).call(|token_sale| token_sale.buy());
            let result = client
                .call_dry_run(&ink_e2e::alice(), &buy_message, 0, None)
                .await
                .return_value();
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "In amount must be in multiples of in_unit".to_string()
                ))
            );

            // when in amount is positive
            // = when amount is not a multiple of in_unit
            let result = client
                .call_dry_run(&ink_e2e::alice(), &buy_message, MOCK_IN_UNIT + 1, None)
                .await
                .return_value();
            assert_eq!(
                result,
                Err(AZTokenSaleError::UnprocessableEntity(
                    "In amount must be in multiples of in_unit".to_string()
                ))
            );
            // = when in amount is a multiple of in_unit
            // == when there is enough stock to fill full order
            let transfer_message = build_message::<ButtonRef>(to_token_id)
                .call(|button| button.transfer(token_sale_id, MOCK_OUT_UNIT * 2, vec![]));
            let transfer_result = client
                .call(&ink_e2e::alice(), transfer_message, 0, None)
                .await
                .unwrap()
                .dry_run
                .exec_result
                .result;
            assert!(transfer_result.is_ok());

            // == * it works
            let original_alice_azero_balance: Balance =
                client.balance(alice_account_id).await.unwrap();
            let buy_message =
                build_message::<AZTokenSaleRef>(token_sale_id).call(|token_sale| token_sale.buy());
            let buy_result = client
                .call(&ink_e2e::bob(), buy_message, MOCK_IN_UNIT, None)
                .await
                .unwrap()
                .dry_run
                .exec_result
                .result;
            assert!(buy_result.is_ok());

            // == * it transfers the out amount to the caller
            let balance_message = build_message::<ButtonRef>(to_token_id)
                .call(|button| button.balance_of(bob_account_id));
            let result = client
                .call_dry_run(&ink_e2e::alice(), &balance_message, 0, None)
                .await
                .return_value();
            assert_eq!(result, MOCK_OUT_UNIT);
            // == * it transfers the in amount to the admin
            assert_eq!(
                client.balance(alice_account_id).await.unwrap(),
                original_alice_azero_balance + MOCK_IN_UNIT
            );

            // == when there is only enough stock to partially fill order
            // == * it works
            let original_token_sale_azero_balance: Balance =
                client.balance(token_sale_id).await.unwrap();
            let buy_message =
                build_message::<AZTokenSaleRef>(token_sale_id).call(|token_sale| token_sale.buy());
            let buy_result = client
                .call(&ink_e2e::bob(), buy_message, MOCK_IN_UNIT * 2, None)
                .await
                .unwrap()
                .dry_run
                .exec_result
                .result;
            assert!(buy_result.is_ok());

            // == * it transfers the available out amount to the caller
            let balance_message = build_message::<ButtonRef>(to_token_id)
                .call(|button| button.balance_of(bob_account_id));
            let result = client
                .call_dry_run(&ink_e2e::alice(), &balance_message, 0, None)
                .await
                .return_value();
            assert_eq!(result, MOCK_OUT_UNIT * 2);

            // == * it transfers the applicable in amount to the admin
            assert_eq!(
                client.balance(alice_account_id).await.unwrap(),
                original_alice_azero_balance + MOCK_IN_UNIT * 2
            );

            // == * it refunds the unused in amount to the buyer
            assert_eq!(
                client.balance(token_sale_id).await.unwrap(),
                original_token_sale_azero_balance
            );

            Ok(())
        }
    }
}
