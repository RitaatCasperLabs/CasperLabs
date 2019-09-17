#![no_std]

extern crate contract_ffi;

use contract_ffi::contract_api;
use contract_ffi::system_contracts::mint;
use contract_ffi::uref::URef;
use contract_ffi::value::account::PurseId;
use contract_ffi::value::U512;

#[repr(u32)]
enum Error {
    PurseNotCreated = 1,
    MintNotFound = 2,
    BalanceNotFound = 3,
    BalanceMismatch = 4,
}

fn mint_purse(amount: U512) -> Result<PurseId, mint::error::Error> {
    let mint = contract_api::get_mint().expect("mint contract should exist");

    let result: Result<URef, mint::error::Error> =
        contract_api::call_contract(mint, &("mint", amount));

    result.map(PurseId::new)
}

#[no_mangle]
pub extern "C" fn call() {
    let amount: U512 = 12345.into();
    let new_purse =
        mint_purse(amount).unwrap_or_else(|_| contract_api::revert(Error::PurseNotCreated as u32));

    let mint = contract_api::get_mint()
        .unwrap_or_else(|| contract_api::revert(Error::MintNotFound as u32));

    let balance: Option<U512> = contract_api::call_contract(mint, &("balance", new_purse));

    match balance {
        None => contract_api::revert(Error::BalanceNotFound as u32),

        Some(balance) if balance == amount => (),

        _ => contract_api::revert(Error::BalanceMismatch as u32),
    }
}
