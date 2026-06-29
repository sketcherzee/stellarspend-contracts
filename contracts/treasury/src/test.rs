use super::{TreasuryContract, TreasuryContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, Address, TreasuryContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    (env, admin, client)
}
#[test]
fn initializes_with_zero_balances() {
    let (_env, _admin, client) = setup();

    assert_eq!(client.get_total_penalties(), 0);
    assert_eq!(client.get_total_fees(), 0);
    assert_eq!(client.get_total_rewards(), 0);
    assert_eq!(client.get_total_reserve(), 0);
}
#[test]
fn credit_penalty_updates_total() {
    let (_env, _admin, client) = setup();

    client.credit_penalty(&100i128);

    assert_eq!(client.get_total_penalties(), 100);
}

#[test]
fn credit_fee_updates_total() {
    let (_env, _admin, client) = setup();

    client.credit_fee(&250i128);

    assert_eq!(client.get_total_fees(), 250);
}

#[test]
fn credit_reward_updates_total() {
    let (_env, _admin, client) = setup();

    client.credit_reward(&500i128);

    assert_eq!(client.get_total_rewards(), 500);
}

#[test]
fn reserve_is_sum_of_all_totals() {
    let (_env, _admin, client) = setup();

    client.credit_penalty(&100i128);
    client.credit_fee(&200i128);
    client.credit_reward(&300i128);

    assert_eq!(client.get_total_reserve(), 600);
}
#[test]
#[should_panic]
fn penalty_rejects_zero_amount() {
    let (_env, _admin, client) = setup();

    client.credit_penalty(&0i128);
}
#[test]
#[should_panic]
fn fee_rejects_negative_amount() {
    let (_env, _admin, client) = setup();

    client.credit_fee(&-50i128);
}

#[test]
#[should_panic]
fn initialize_cannot_be_called_twice() {
    let (env, admin, client) = setup();

    client.initialize(&admin);
}