//! Core reward account management logic.
//!
//! This module contains the business logic for reward account lifecycle
//! operations. Contract entry points in `lib.rs` delegate to these functions
//! after performing authorisation checks.

use soroban_sdk::{Address, Env};

use crate::storage::{
    set_lifetime_claimed, set_lifetime_earned, set_reward_account, set_reward_balance,
};
use crate::types::RewardAccount;
use crate::validation::{validate_account_not_registered, validate_contract_initialized};
use crate::RewardsError;

/// Registers a new reward account for `participant` with zeroed default values.
///
/// Creates and persists a [`RewardAccount`] record together with the individual
/// scalar storage entries (`RewardBalance`, `LifetimeEarned`, `LifetimeClaimed`)
/// so that every storage helper returns a consistent `0` immediately after
/// registration.
///
/// # Errors
/// - `NotInitialized` — contract has not been initialised.
/// - `AccountAlreadyRegistered` — an account already exists for `participant`.
pub fn register_reward_account(env: &Env, participant: &Address) -> Result<(), RewardsError> {
    validate_contract_initialized(env)?;
    validate_account_not_registered(env, participant)?;

    let now = env.ledger().sequence() as u64;

    let account = RewardAccount {
        owner: participant.clone(),
        balance: 0,
        lifetime_earned: 0,
        lifetime_claimed: 0,
        created_at: now,
        last_updated: now,
    };

    set_reward_account(env, participant, &account);
    set_reward_balance(env, participant, 0);
    set_lifetime_earned(env, participant, 0);
    set_lifetime_claimed(env, participant, 0);

    env.events()
        .publish(("rewards", "account_registered"), participant.clone());

    Ok(())
}
