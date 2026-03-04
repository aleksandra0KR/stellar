#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String};

fn create_token<'a>(e: &Env, admin: &Address) -> NabokaContractClient<'a> {
    let id = e.register(
        NabokaContract,
        (
            admin,
            7u32,
            String::from_str(e, "NabokaToken"),
            String::from_str(e, "NT"),
        ),
    );
    NabokaContractClient::new(e, &id)
}

#[test]
fn test_metadata() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let t = create_token(&e, &admin);

    assert_eq!(t.name(), String::from_str(&e, "NabokaToken"));
    assert_eq!(t.symbol(), String::from_str(&e, "NT"));
    assert_eq!(t.decimals(), 7);
    assert_eq!(t.admin(), admin);
}

#[test]
fn test_mint_and_balance() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let t = create_token(&e, &admin);

    t.mint(&user, &1000);
    assert_eq!(t.balance(&user), 1000);
}

#[test]
fn test_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let u1 = Address::generate(&e);
    let u2 = Address::generate(&e);
    let t = create_token(&e, &admin);

    t.mint(&u1, &1000);
    t.transfer(&u1, &u2, &300);
    assert_eq!(t.balance(&u1), 700);
    assert_eq!(t.balance(&u2), 300);
}

#[test]
fn test_burn() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let t = create_token(&e, &admin);

    t.mint(&user, &1000);
    t.burn(&user, &400);
    assert_eq!(t.balance(&user), 600);
}

