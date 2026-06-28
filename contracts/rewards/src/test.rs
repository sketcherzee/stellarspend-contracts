#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{
    storage::{
        get_lifetime_claimed, get_lifetime_earned, get_reward_account, get_reward_balance,
        has_reward_account, set_lifetime_claimed, set_lifetime_earned, set_reward_account,
        set_reward_balance,
    },
    types::{RewardAccount, RewardStatus, RewardTransaction, RewardType},
    RewardsContract, RewardsContractClient,
};

fn setup() -> (Env, Address, RewardsContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    let client = RewardsContractClient::new(&env, &contract_id);
    (env, admin, client)
}

// ── Contract entry-point tests (from #875) ────────────────────────────────────

#[test]
fn test_initialize_sets_admin() {
    let (_env, admin, client) = setup();
    client.initialize(&admin);
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_is_initialized_returns_true_after_init() {
    let (_env, admin, client) = setup();
    assert!(!client.is_initialized());
    client.initialize(&admin);
    assert!(client.is_initialized());
}

#[test]
#[should_panic]
fn test_double_initialize_panics() {
    let (_env, admin, client) = setup();
    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
#[should_panic]
fn test_get_admin_before_init_panics() {
    let (_env, _admin, client) = setup();
    client.get_admin();
}

// ── Storage helper tests (#876) ───────────────────────────────────────────────
//
// Storage helpers must be invoked from within a contract context.
// We use `env.as_contract(&contract_id, || { ... })` to satisfy that
// requirement without needing a dedicated accessor entry point on the contract.

#[test]
fn test_reward_balance_defaults_to_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        assert_eq!(get_reward_balance(&env, &user), 0);
    });
}

#[test]
fn test_set_and_get_reward_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        set_reward_balance(&env, &user, 5_000_000);
        assert_eq!(get_reward_balance(&env, &user), 5_000_000);
    });
}

#[test]
fn test_reward_balance_overwrite() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        set_reward_balance(&env, &user, 1_000);
        set_reward_balance(&env, &user, 9_999);
        assert_eq!(get_reward_balance(&env, &user), 9_999);
    });
}

#[test]
fn test_lifetime_earned_defaults_to_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        assert_eq!(get_lifetime_earned(&env, &user), 0);
    });
}

#[test]
fn test_set_and_get_lifetime_earned() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        set_lifetime_earned(&env, &user, 100_000_000);
        assert_eq!(get_lifetime_earned(&env, &user), 100_000_000);
    });
}

#[test]
fn test_lifetime_claimed_defaults_to_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        assert_eq!(get_lifetime_claimed(&env, &user), 0);
    });
}

#[test]
fn test_set_and_get_lifetime_claimed() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        set_lifetime_claimed(&env, &user, 50_000_000);
        assert_eq!(get_lifetime_claimed(&env, &user), 50_000_000);
    });
}

#[test]
fn test_has_reward_account_false_before_creation() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        assert!(!has_reward_account(&env, &user));
    });
}

#[test]
fn test_set_and_get_reward_account() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());

    env.as_contract(&contract_id, || {
        let record = RewardAccount {
            owner: user.clone(),
            balance: 2_000_000,
            lifetime_earned: 10_000_000,
            lifetime_claimed: 8_000_000,
            created_at: 100,
            last_updated: 200,
        };

        set_reward_account(&env, &user, &record);
        assert!(has_reward_account(&env, &user));

        let fetched = get_reward_account(&env, &user).expect("account should exist");
        assert_eq!(fetched.owner, user);
        assert_eq!(fetched.balance, 2_000_000);
        assert_eq!(fetched.lifetime_earned, 10_000_000);
        assert_eq!(fetched.lifetime_claimed, 8_000_000);
        assert_eq!(fetched.created_at, 100);
        assert_eq!(fetched.last_updated, 200);
    });
}

#[test]
fn test_reward_account_returns_none_when_absent() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        assert!(get_reward_account(&env, &user).is_none());
    });
}

#[test]
fn test_balances_are_independent_per_user() {
    let env = Env::default();
    env.mock_all_auths();
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let contract_id = env.register(RewardsContract, ());
    env.as_contract(&contract_id, || {
        set_reward_balance(&env, &user_a, 1_000);
        set_reward_balance(&env, &user_b, 2_000);
        assert_eq!(get_reward_balance(&env, &user_a), 1_000);
        assert_eq!(get_reward_balance(&env, &user_b), 2_000);
    });
}

// ── Reward account registration tests (#878) ─────────────────────────────────

#[test]
fn test_register_account_succeeds() {
    let (env, admin, client) = setup();
    client.initialize(&admin);
    let user = Address::generate(&env);
    client.register_account(&user);
    assert!(client.get_account(&user).is_some());
}

#[test]
fn test_register_account_stores_correct_defaults() {
    let (env, admin, client) = setup();
    client.initialize(&admin);
    let user = Address::generate(&env);
    client.register_account(&user);

    let account = client.get_account(&user).expect("account should exist");
    assert_eq!(account.owner, user);
    assert_eq!(account.balance, 0);
    assert_eq!(account.lifetime_earned, 0);
    assert_eq!(account.lifetime_claimed, 0);
}

#[test]
fn test_register_account_sets_timestamps() {
    let (env, admin, client) = setup();
    client.initialize(&admin);
    let user = Address::generate(&env);
    client.register_account(&user);

    let account = client.get_account(&user).expect("account should exist");
    assert_eq!(account.created_at, account.last_updated);
}

#[test]
#[should_panic]
fn test_duplicate_registration_panics() {
    let (env, admin, client) = setup();
    client.initialize(&admin);
    let user = Address::generate(&env);
    client.register_account(&user);
    client.register_account(&user);
}

#[test]
#[should_panic]
fn test_register_account_before_init_panics() {
    let (env, _admin, client) = setup();
    let user = Address::generate(&env);
    client.register_account(&user);
}

#[test]
fn test_get_account_returns_none_for_unregistered() {
    let (env, admin, client) = setup();
    client.initialize(&admin);
    let user = Address::generate(&env);
    assert!(client.get_account(&user).is_none());
}

#[test]
fn test_multiple_accounts_are_independent() {
    let (env, admin, client) = setup();
    client.initialize(&admin);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    client.register_account(&user_a);
    client.register_account(&user_b);

    let a = client.get_account(&user_a).expect("user_a should exist");
    let b = client.get_account(&user_b).expect("user_b should exist");
    assert_eq!(a.owner, user_a);
    assert_eq!(b.owner, user_b);
    assert_ne!(a.owner, b.owner);
}

// ── Data model tests (#877) ───────────────────────────────────────────────────

#[test]
fn test_reward_type_variants_are_distinct() {
    assert_ne!(RewardType::SpendingLimit, RewardType::SavingsGoal);
    assert_ne!(RewardType::SavingsGoal, RewardType::Streak);
    assert_ne!(RewardType::Streak, RewardType::Referral);
    assert_ne!(RewardType::Referral, RewardType::ManualGrant);
}

#[test]
fn test_reward_type_clone() {
    let rt = RewardType::SavingsGoal;
    let cloned = rt.clone();
    assert_eq!(rt, cloned);
}

#[test]
fn test_reward_status_variants_are_distinct() {
    assert_ne!(RewardStatus::Pending, RewardStatus::Confirmed);
    assert_ne!(RewardStatus::Confirmed, RewardStatus::Claimed);
    assert_ne!(RewardStatus::Claimed, RewardStatus::Cancelled);
    assert_ne!(RewardStatus::Pending, RewardStatus::Cancelled);
}

#[test]
fn test_reward_status_clone() {
    let s = RewardStatus::Confirmed;
    assert_eq!(s.clone(), RewardStatus::Confirmed);
}

#[test]
fn test_reward_status_pending_is_not_claimed() {
    let status = RewardStatus::Pending;
    assert_ne!(status, RewardStatus::Claimed);
}

#[test]
fn test_reward_transaction_fields_are_correct() {
    let env = Env::default();
    let recipient = Address::generate(&env);

    let tx = RewardTransaction {
        id: 42,
        recipient: recipient.clone(),
        amount: 1_000_000,
        reward_type: RewardType::Streak,
        status: RewardStatus::Confirmed,
        created_at: 500,
        updated_at: 600,
    };

    assert_eq!(tx.id, 42);
    assert_eq!(tx.recipient, recipient);
    assert_eq!(tx.amount, 1_000_000);
    assert_eq!(tx.reward_type, RewardType::Streak);
    assert_eq!(tx.status, RewardStatus::Confirmed);
    assert_eq!(tx.created_at, 500);
    assert_eq!(tx.updated_at, 600);
}

#[test]
fn test_reward_transaction_clone() {
    let env = Env::default();
    let recipient = Address::generate(&env);

    let tx = RewardTransaction {
        id: 1,
        recipient: recipient.clone(),
        amount: 500_000,
        reward_type: RewardType::Referral,
        status: RewardStatus::Pending,
        created_at: 100,
        updated_at: 0,
    };

    let cloned = tx.clone();
    assert_eq!(cloned.id, tx.id);
    assert_eq!(cloned.amount, tx.amount);
    assert_eq!(cloned.reward_type, RewardType::Referral);
    assert_eq!(cloned.status, RewardStatus::Pending);
    assert_eq!(cloned.updated_at, 0);
}

#[test]
fn test_reward_transaction_status_transition() {
    let env = Env::default();
    let recipient = Address::generate(&env);

    let mut tx = RewardTransaction {
        id: 10,
        recipient: recipient.clone(),
        amount: 250_000,
        reward_type: RewardType::ManualGrant,
        status: RewardStatus::Pending,
        created_at: 200,
        updated_at: 0,
    };

    assert_eq!(tx.status, RewardStatus::Pending);
    tx.status = RewardStatus::Confirmed;
    assert_eq!(tx.status, RewardStatus::Confirmed);
    tx.status = RewardStatus::Claimed;
    assert_eq!(tx.status, RewardStatus::Claimed);
}

#[test]
fn test_all_reward_types_can_be_used_in_transaction() {
    let env = Env::default();
    let recipient = Address::generate(&env);

    let types = [
        RewardType::SpendingLimit,
        RewardType::SavingsGoal,
        RewardType::Streak,
        RewardType::Referral,
        RewardType::ManualGrant,
    ];

    for reward_type in types {
        let tx = RewardTransaction {
            id: 1,
            recipient: recipient.clone(),
            amount: 100,
            reward_type: reward_type.clone(),
            status: RewardStatus::Pending,
            created_at: 1,
            updated_at: 0,
        };
        assert_eq!(tx.reward_type, reward_type);
    }
}
