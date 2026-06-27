//! # Allowances Contract
//!
//! Manages recurring spending allowances on Stellar/Soroban.
//!
//! ## Issues resolved
//! - #822 Create Allowance Contract — storage schema + contract scaffold
//! - #823 Add Allowance Creation    — `create_allowance` with event emission
//! - #824 Implement Weekly Allowances  — `Frequency::Weekly` (7-day interval)
//! - #825 Implement Monthly Allowances — `Frequency::Monthly` (30-day interval)
//! - #832 Add Daily Allowances         — `Frequency::Daily` (24-hour interval)
//! - #833 Add Allowance Pause/Resume   — `pause_allowance` / `resume_allowance`
//! - #834 Add Allowance Cancellation   — `cancel_allowance` (already present, confirmed)
//! - #835 Add Allowance Beneficiary Update — `update_beneficiary`
//! - #847 Optimize Allowance Storage    — shared `load`/`save`/`append_index` helpers (one accessor per op)

#![no_std]

mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, token, Address, Env, Vec,
};

use types::{AllowanceError, Allowance, DataKey, Frequency};

// ── Internal storage helpers (#847) ───────────────────────────────────────────
//
// Centralize allowance reads/writes so every operation performs exactly one
// load and one store, and the persistent-storage accessor + key are built once
// per access rather than being duplicated across call sites. Previously each
// mutator re-derived the `env.storage().persistent()` accessor for both its
// read and its write (and index updates did so twice); these helpers collapse
// that to a single accessor per logical operation.

/// Loads an allowance by id, panicking with `NotFound` if it does not exist.
fn load_allowance(env: &Env, allowance_id: u64) -> Allowance {
    env.storage()
        .persistent()
        .get(&DataKey::Allowance(allowance_id))
        .unwrap_or_else(|| panic_with_error!(env, AllowanceError::NotFound))
}

/// Persists an allowance record.
fn save_allowance(env: &Env, allowance_id: u64, allowance: &Allowance) {
    env.storage()
        .persistent()
        .set(&DataKey::Allowance(allowance_id), allowance);
}

/// Appends an id to an index vector (`OwnerAllowances` / `RecipientAllowances`)
/// using a single storage accessor for the read-modify-write.
fn append_index(env: &Env, key: DataKey, allowance_id: u64) {
    let store = env.storage().persistent();
    let mut ids: Vec<u64> = store.get(&key).unwrap_or(Vec::new(env));
    ids.push_back(allowance_id);
    store.set(&key, &ids);
}

#[contract]
pub struct AllowancesContract;

#[contractimpl]
impl AllowancesContract {
    // ── Creation ──────────────────────────────────────────────────────────

    pub fn create_allowance(
        env: Env,
        owner: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        frequency: Frequency,
        start_time: u64,
    ) -> u64 {
        owner.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, AllowanceError::InvalidAmount);
        }

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AllowanceCount)
            .unwrap_or(0);
        count += 1;

        let allowance = Allowance {
            owner: owner.clone(),
            recipient: recipient.clone(),
            token,
            amount,
            frequency: frequency.clone(),
            next_distribution: start_time,
            distribution_count: 0,
            active: true,
            paused: false,
        };

        save_allowance(&env, count, &allowance);
        env.storage().instance().set(&DataKey::AllowanceCount, &count);

        append_index(&env, DataKey::OwnerAllowances(owner.clone()), count);
        append_index(&env, DataKey::RecipientAllowances(recipient.clone()), count);

        let freq_tag = match &frequency {
            Frequency::Once    => symbol_short!("once"),
            Frequency::Daily   => symbol_short!("daily"),
            Frequency::Weekly  => symbol_short!("weekly"),
            Frequency::Monthly => symbol_short!("monthly"),
        };
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("created"), count),
            (owner, recipient, amount, freq_tag),
        );

        count
    }

    // ── Distribution ──────────────────────────────────────────────────────

    pub fn distribute(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        if !allowance.active {
            panic_with_error!(&env, AllowanceError::AlreadyInactive);
        }
        if allowance.paused {
            panic_with_error!(&env, AllowanceError::Paused);
        }

        let now = env.ledger().timestamp();
        if now < allowance.next_distribution {
            panic_with_error!(&env, AllowanceError::TooEarlyToDistribute);
        }

        let token_client = token::Client::new(&env, &allowance.token);
        if token_client.balance(&allowance.owner) < allowance.amount {
            panic_with_error!(&env, AllowanceError::InsufficientBalance);
        }

        token_client.transfer_from(
            &env.current_contract_address(),
            &allowance.owner,
            &allowance.recipient,
            &allowance.amount,
        );

        allowance.distribution_count += 1;

        match allowance.frequency.interval_seconds() {
            None => {
                allowance.active = false;
                allowance.next_distribution = 0;
            }
            Some(interval) => {
                allowance.next_distribution += interval;
                if allowance.next_distribution <= now {
                    let missed = (now - allowance.next_distribution) / interval;
                    allowance.next_distribution += (missed + 1) * interval;
                }
            }
        }

        save_allowance(&env, allowance_id, &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("distrib"), allowance_id),
            (allowance.recipient, allowance.amount, allowance.next_distribution),
        );
    }

    // ── Pause / Resume (#833) ─────────────────────────────────────────────

    /// Temporarily suspends distributions. Only the owner may pause.
    pub fn pause_allowance(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active  { panic_with_error!(&env, AllowanceError::AlreadyInactive); }
        if allowance.paused   { panic_with_error!(&env, AllowanceError::AlreadyPaused); }

        allowance.paused = true;
        save_allowance(&env, allowance_id, &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("paused"), allowance_id),
            allowance.owner,
        );
    }

    /// Resumes a paused allowance. Only the owner may resume.
    pub fn resume_allowance(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active  { panic_with_error!(&env, AllowanceError::AlreadyInactive); }
        if !allowance.paused  { panic_with_error!(&env, AllowanceError::NotPaused); }

        allowance.paused = false;
        save_allowance(&env, allowance_id, &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("resumed"), allowance_id),
            allowance.owner,
        );
    }

    // ── Cancellation (#834) ───────────────────────────────────────────────

    /// Permanently cancels an allowance. Only the owner may cancel.
    pub fn cancel_allowance(env: Env, allowance_id: u64) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active { panic_with_error!(&env, AllowanceError::AlreadyInactive); }

        allowance.active = false;
        save_allowance(&env, allowance_id, &allowance);
        env.events().publish(
            (symbol_short!("allow"), symbol_short!("canceled"), allowance_id),
            allowance.owner,
        );
    }

    // ── Beneficiary update (#835) ─────────────────────────────────────────

    /// Updates the recipient of an active allowance. Only the owner may call.
    /// Future distributions go to `new_recipient`; history is preserved.
    pub fn update_beneficiary(env: Env, allowance_id: u64, new_recipient: Address) {
        let mut allowance = load_allowance(&env, allowance_id);

        allowance.owner.require_auth();
        if !allowance.active { panic_with_error!(&env, AllowanceError::AlreadyInactive); }

        let old_recipient = allowance.recipient.clone();
        allowance.recipient = new_recipient.clone();
        save_allowance(&env, allowance_id, &allowance);

        // Update recipient index for new beneficiary
        append_index(&env, DataKey::RecipientAllowances(new_recipient.clone()), allowance_id);

        env.events().publish(
            (symbol_short!("allow"), symbol_short!("ben_upd"), allowance_id),
            (old_recipient, new_recipient),
        );
    }

    // ── Queries ───────────────────────────────────────────────────────────

    pub fn get_allowance(env: Env, allowance_id: u64) -> Allowance {
        load_allowance(&env, allowance_id)
    }

    pub fn get_owner_allowances(env: Env, owner: Address) -> Vec<u64> {
        env.storage().persistent()
            .get(&DataKey::OwnerAllowances(owner))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_recipient_allowances(env: Env, recipient: Address) -> Vec<u64> {
        env.storage().persistent()
            .get(&DataKey::RecipientAllowances(recipient))
            .unwrap_or(Vec::new(&env))
    }

    pub fn allowance_count(env: Env) -> u64 {
        env.storage().instance()
            .get(&DataKey::AllowanceCount)
            .unwrap_or(0)
    }
}
