# Implement Reward Initialization Logic

**Closes #878**

---

## Summary

Implements the reward account registration flow — the entry point through which any participant can join the reward system for the first time. A participant self-registers by calling `register_account`, which writes a zeroed `RewardAccount` record and all scalar storage entries. Duplicate registrations are rejected with a typed error. A dedicated `validation.rs` module enforces all pre-conditions, and a `rewards.rs` module contains the business logic cleanly separated from the contract interface.

This PR builds on #875 (bootstrap), #876 (storage schema), and #877 (data models).

---

## Changes

### New: `contracts/rewards/src/validation.rs`

Two pre-condition validators used before any state-mutating operation:

| Function | Description |
|---|---|
| `validate_contract_initialized` | Returns `NotInitialized` if the contract has not been initialised |
| `validate_account_not_registered` | Returns `AccountAlreadyRegistered` if a record already exists for the address |

### New: `contracts/rewards/src/rewards.rs`

Core business logic for reward account lifecycle operations.

| Function | Description |
|---|---|
| `register_reward_account` | Validates preconditions, writes a zeroed `RewardAccount` and all scalar entries, emits an `account_registered` event |

### Updated: `contracts/rewards/src/lib.rs`

- Declared `pub mod rewards` and `pub mod validation`
- Added `RewardsError::AccountAlreadyRegistered = 4` error variant
- Added `register_account(env, participant)` contract entry point — caller self-authorises
- Added `get_account(env, participant)` read-only entry point returning `Option<RewardAccount>`

### Updated: `contracts/rewards/src/test.rs`

Added 7 new registration tests (on top of 24 existing from #875–#877):

| Test | What it covers |
|---|---|
| `test_register_account_succeeds` | Happy path — account is created and retrievable |
| `test_register_account_stores_correct_defaults` | All fields initialised to zero |
| `test_register_account_sets_timestamps` | `created_at` equals `last_updated` on creation |
| `test_duplicate_registration_panics` | Second registration for same address panics |
| `test_register_account_before_init_panics` | Registration before contract init panics |
| `test_get_account_returns_none_for_unregistered` | Unregistered address returns `None` |
| `test_multiple_accounts_are_independent` | Two users register without interfering with each other |

---

## Default Values on Registration

| Field | Initial value |
|---|---|
| `balance` | `0` |
| `lifetime_earned` | `0` |
| `lifetime_claimed` | `0` |
| `created_at` | Current ledger sequence |
| `last_updated` | Current ledger sequence |

---

## Test Results

```
running 31 tests
test test::test_initialize_sets_admin ... ok
test test::test_is_initialized_returns_true_after_init ... ok
test test::test_double_initialize_panics - should panic ... ok
test test::test_get_admin_before_init_panics - should panic ... ok
test test::test_reward_balance_defaults_to_zero ... ok
test test::test_set_and_get_reward_balance ... ok
test test::test_reward_balance_overwrite ... ok
test test::test_lifetime_earned_defaults_to_zero ... ok
test test::test_set_and_get_lifetime_earned ... ok
test test::test_lifetime_claimed_defaults_to_zero ... ok
test test::test_set_and_get_lifetime_claimed ... ok
test test::test_has_reward_account_false_before_creation ... ok
test test::test_set_and_get_reward_account ... ok
test test::test_reward_account_returns_none_when_absent ... ok
test test::test_balances_are_independent_per_user ... ok
test test::test_register_account_succeeds ... ok
test test::test_register_account_stores_correct_defaults ... ok
test test::test_register_account_sets_timestamps ... ok
test test::test_duplicate_registration_panics - should panic ... ok
test test::test_register_account_before_init_panics - should panic ... ok
test test::test_get_account_returns_none_for_unregistered ... ok
test test::test_multiple_accounts_are_independent ... ok
test test::test_reward_type_variants_are_distinct ... ok
test test::test_reward_type_clone ... ok
test test::test_reward_status_variants_are_distinct ... ok
test test::test_reward_status_clone ... ok
test test::test_reward_status_pending_is_not_claimed ... ok
test test::test_reward_transaction_fields_are_correct ... ok
test test::test_reward_transaction_clone ... ok
test test::test_reward_transaction_status_transition ... ok
test test::test_all_reward_types_can_be_used_in_transaction ... ok

test result: ok. 31 passed; 0 failed; 0 ignored
```

---

## Out of Scope

This PR implements account registration only. Reward issuance (crediting balances) and claiming (transferring tokens) are deferred to follow-on issues.
