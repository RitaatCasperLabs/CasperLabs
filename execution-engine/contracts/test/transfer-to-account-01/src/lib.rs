#![no_std]

use contract_ffi::{
    contract_api::{runtime, system, Error},
    unwrap_or_revert::UnwrapOrRevert,
    value::U512,
};

const TRANSFER_AMOUNT: u32 = 250_000_000 + 1000;

#[no_mangle]
pub extern "C" fn call() {
    let public_key = runtime::get_arg(0)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let amount = U512::from(TRANSFER_AMOUNT);

    let result = system::transfer_to_account(public_key, amount);

    assert!(result.is_ok());
}
