#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token::{self, Interface as _}, Address, Env, String,
};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

// ── Константы TTL ──

const INSTANCE_BUMP: u32 = 7 * 17280;
const INSTANCE_THRESHOLD: u32 = 6 * 17280;
const BALANCE_BUMP: u32 = 30 * 17280;
const BALANCE_THRESHOLD: u32 = 29 * 17280;


#[contracttype]
pub enum DataKey {
    Admin,
    Balance(Address),
    Allowance(AllowanceKey),
}

#[contracttype]
#[derive(Clone)]
pub struct AllowanceKey {
    pub from: Address,
    pub spender: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct AllowanceVal {
    pub amount: i128,
    pub expiration_ledger: u32,
}

fn check_positive(amount: i128) {
    if amount < 0 {
        panic!("negative amount");
    }
}

fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

fn get_balance(e: &Env, addr: &Address) -> i128 {
    let key = DataKey::Balance(addr.clone());
    if let Some(b) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_THRESHOLD, BALANCE_BUMP);
        b
    } else {
        0
    }
}

fn set_balance(e: &Env, addr: &Address, amount: i128) {
    let key = DataKey::Balance(addr.clone());
    e.storage().persistent().set(&key, &amount);
    e.storage().persistent().extend_ttl(&key, BALANCE_THRESHOLD, BALANCE_BUMP);
}

fn get_allowance(e: &Env, from: &Address, spender: &Address) -> AllowanceVal {
    let key = DataKey::Allowance(AllowanceKey {
        from: from.clone(),
        spender: spender.clone(),
    });
    if let Some(a) = e.storage().persistent().get::<_, AllowanceVal>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_THRESHOLD, BALANCE_BUMP);
        if a.expiration_ledger < e.ledger().sequence() {
            AllowanceVal { amount: 0, expiration_ledger: 0 }
        } else {
            a
        }
    } else {
        AllowanceVal { amount: 0, expiration_ledger: 0 }
    }
}

fn set_allowance(e: &Env, from: &Address, spender: &Address, amount: i128, exp: u32) {
    let key = DataKey::Allowance(AllowanceKey {
        from: from.clone(),
        spender: spender.clone(),
    });
    let val = AllowanceVal { amount, expiration_ledger: exp };
    e.storage().persistent().set(&key, &val);
    if amount > 0 {
        e.storage().persistent().extend_ttl(&key, BALANCE_THRESHOLD, BALANCE_BUMP);
    }
}


fn spend_allowance(e: &Env, from: &Address, spender: &Address, amount: i128) {
    let a = get_allowance(e, from, spender);
    if a.amount < amount {
        panic!("insufficient allowance");
    }
    set_allowance(e, from, spender, a.amount - amount, a.expiration_ledger);
}

fn bump(e: &Env) {
    e.storage().instance().extend_ttl(INSTANCE_THRESHOLD, INSTANCE_BUMP);
}

// ── Контракт ──

#[contract]
pub struct NabokaContract;

#[contractimpl]
impl NabokaContract {
    pub fn __constructor(e: Env, admin: Address, decimal: u32, name: String, symbol: String) {
        if decimal > 18 {
            panic!("decimal > 18");
        }
        e.storage().instance().set(&DataKey::Admin, &admin);
        TokenUtils::new(&e).metadata().set_metadata(&TokenMetadata {
            decimal,
            name,
            symbol,
        });
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        check_positive(amount);
        let admin = get_admin(&e);
        admin.require_auth();
        bump(&e);
        set_balance(&e, &to, get_balance(&e, &to) + amount);
        TokenUtils::new(&e).events().mint(admin, to, amount);
    }

    pub fn admin(e: Env) -> Address {
        get_admin(&e)
    }
}

#[contractimpl]
impl token::Interface for NabokaContract {
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        bump(&e);
        get_allowance(&e, &from, &spender).amount
    }

    fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        from.require_auth();
        check_positive(amount);
        bump(&e);
        set_allowance(&e, &from, &spender, amount, expiration_ledger);
        TokenUtils::new(&e).events().approve(from, spender, amount, expiration_ledger);
    }

    fn balance(e: Env, id: Address) -> i128 {
        bump(&e);
        get_balance(&e, &id)
    }

    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        check_positive(amount);
        bump(&e);
        let fb = get_balance(&e, &from);
        if fb < amount { panic!("insufficient balance"); }
        set_balance(&e, &from, fb - amount);
        set_balance(&e, &to, get_balance(&e, &to) + amount);
        TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        check_positive(amount);
        bump(&e);
        spend_allowance(&e, &from, &spender, amount);
        let fb = get_balance(&e, &from);
        if fb < amount { panic!("insufficient balance"); }
        set_balance(&e, &from, fb - amount);
        set_balance(&e, &to, get_balance(&e, &to) + amount);
        TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn burn(e: Env, from: Address, amount: i128) {
        from.require_auth();
        check_positive(amount);
        bump(&e);
        let b = get_balance(&e, &from);
        if b < amount { panic!("insufficient balance"); }
        set_balance(&e, &from, b - amount);
        TokenUtils::new(&e).events().burn(from, amount);
    }

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        check_positive(amount);
        bump(&e);
        spend_allowance(&e, &from, &spender, amount);
        let b = get_balance(&e, &from);
        if b < amount { panic!("insufficient balance"); }
        set_balance(&e, &from, b - amount);
        TokenUtils::new(&e).events().burn(from, amount);
    }

    fn decimals(e: Env) -> u32 {
        TokenUtils::new(&e).metadata().get_metadata().decimal
    }

    fn name(e: Env) -> String {
        TokenUtils::new(&e).metadata().get_metadata().name
    }

    fn symbol(e: Env) -> String {
        TokenUtils::new(&e).metadata().get_metadata().symbol
    }
}

mod test;