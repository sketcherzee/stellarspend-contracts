//! Validation helpers for the rewards contract.
//!
//! These functions are called before any state-mutating operation to ensure
//! pre-conditions are met and meaningful errors are returned to the caller.

use soroban_sdk::{Address, Env};

use crate::storage::has_reward_account;
use crate::types::DataKey;
use crate::RewardsError;

/// Asserts that no reward account exists yet for `account`.
///
/// # Errors
/// Returns `RewardsError::AccountAlreadyRegistered` if a record already exists.
pub fn validate_account_not_registered(env: &Env, account: &Address) -> Result<(), RewardsError> {
    if has_reward_account(env, account) {
        return Err(RewardsError::AccountAlreadyRegistered);
    }
    Ok(())
}

/// Asserts that the contract has been initialised.
///
/// # Errors
/// Returns `RewardsError::NotInitialized` if the contract is uninitialised.
pub fn validate_contract_initialized(env: &Env) -> Result<(), RewardsError> {
    if !env.storage().instance().has(&DataKey::Initialized) {
        return Err(RewardsError::NotInitialized);
    }
    Ok(())
}
