#![no_std]

#[macro_use]
extern crate alloc;
extern crate contract_ffi;

use contract_ffi::contract_api::{self, Error as ApiError};
use contract_ffi::key::Key;
use contract_ffi::system_contracts::mint;
use contract_ffi::uref::URef;
use contract_ffi::value::account::PurseId;
use contract_ffi::value::U512;

enum Error {
    PurseNotCreated = ApiError::unreserved_min_plus(0),
    BalanceNotFound = ApiError::unreserved_min_plus(1),
    BalanceMismatch = ApiError::unreserved_min_plus(2),
}

fn mint_purse(amount: U512) -> Result<PurseId, mint::error::Error> {
    let mint = contract_api::get_mint();

    let result: Result<URef, mint::error::Error> =
        contract_api::call_contract(mint, &("mint", amount), &vec![]);

    result.map(PurseId::new)
}

#[no_mangle]
pub extern "C" fn call() {
    let amount: U512 = 12345.into();
    let new_purse =
        mint_purse(amount).unwrap_or_else(|_| contract_api::revert(Error::PurseNotCreated as u32));

    let mint = contract_api::get_mint();

    let balance: Option<U512> = contract_api::call_contract(
        mint,
        &("balance", new_purse),
        &vec![Key::URef(new_purse.value())],
    );

    match balance {
        None => contract_api::revert(Error::BalanceNotFound as u32),

        Some(balance) if balance == amount => (),

        _ => contract_api::revert(Error::BalanceMismatch as u32),
    }
}
