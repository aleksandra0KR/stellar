## Настройка окружения
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
```
source "$HOME/.cargo/env"
```
```
rustc --version
```
```
cargo --version
```

```
rustup target add wasm32v1-none
```
```
cargo install stellar-cli --features opt
```
```
stellar --version
```
---

## Генерация ключей и получение тестовых средств
```
stellar keys generate --global naboka --network testnet --fund
```
```
stellar keys address naboka
```

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/1.png?raw=true?raw=true)


---

## Инициализация проекта

```
stellar contract init naboka-token
```
```
cd naboka-token
```
```
mv contracts/hello-world contracts/naboka_contract
```

---

## Файлы

 naboka-token/Cargo.toml:

```
[workspace]
resolver = "2"
members = [
"contracts/*",
]

[workspace.dependencies]
soroban-sdk = "22"
soroban-token-sdk = "22"

[profile.release]
opt-level = "z"
overflow-checks = true
debug = 0
strip = "symbols"
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = true
```

---

contracts/naboka_contract/Cargo.toml

```
[package]
name = "naboka-contract"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]
doctest = false

[dependencies]
soroban-sdk = { workspace = true }
soroban-token-sdk = { workspace = true }

[dev-dependencies]
soroban-sdk = { workspace = true, features = ["testutils"] }

```

---

## Контракт contracts/naboka_contract/src/lib.rs

```
#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token::{self, Interface as _}, Address, Env, String,
};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;


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

```


## Тесты contracts/naboka_contract/src/test.rs

```
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

```

## Прогон тестов
```
cargo test
```

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/2.png?raw=true)
---

## Сборка

```
stellar contract build
```

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/3.png?raw=true)


## Шаг 11. Развёртывание в тестовой сети

```
stellar contract deploy \
--wasm target/wasm32v1-none/release/naboka_contract.wasm \
--source-account naboka \
--network testnet \
--alias naboka_token \
-- \
--admin "$(stellar keys address naboka)" \
--decimal 7 \
--name "NabokaToken" \
--symbol "NT"
```

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/4.png?raw=true)

Сохраняем contract id

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/5.png?raw=true)
---

## Эмиссия
```
stellar contract invoke \
--id "$CONTRACT" \
--source-account naboka \
--network testnet \
-- \
mint \
--to "$(stellar keys address naboka)" \
--amount 10000
```

Проверка баланса:
```
stellar contract invoke \
--id "$CONTRACT" \
--source-account naboka \
--network testnet \
-- \
balance \
--id "$(stellar keys address naboka)"
```
![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/6.png?raw=true)

## Перевод 

Перевели на свой счет

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/7.png?raw=true)


Баланс у нас 500 у набоки на 500 меньше

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/8.png?raw=true)

---

## C одного аккаунта одногруппника  на другой аккаунт одногрупника

![alt text](https://github.com/aleksandra0KR/stellar/blob/main/img/9.png?raw=true)



# https://stellar.expert/explorer/testnet/contract/CD7TWFKVWR4ATBJXJH3ONE6XG2X76DAVYPH55K7HUC3MTZGFR6XDBX7R
