use ink::{
    env::Error as InkEnvError,
    prelude::{format, string::String},
    LangError,
};
use openbrush::contracts::psp22::PSP22Error;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AZTokenSaleError {
    ContractCall(LangError),
    InkEnvError(String),
    PSP22Error(PSP22Error),
    UnprocessableEntity(String),
}
impl From<InkEnvError> for AZTokenSaleError {
    fn from(e: InkEnvError) -> Self {
        AZTokenSaleError::InkEnvError(format!("{e:?}"))
    }
}
impl From<LangError> for AZTokenSaleError {
    fn from(e: LangError) -> Self {
        AZTokenSaleError::ContractCall(e)
    }
}
impl From<PSP22Error> for AZTokenSaleError {
    fn from(e: PSP22Error) -> Self {
        AZTokenSaleError::PSP22Error(e)
    }
}
