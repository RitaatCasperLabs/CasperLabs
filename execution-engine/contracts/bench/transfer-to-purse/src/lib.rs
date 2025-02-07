#![no_std]

extern crate contract_ffi;

use contract_ffi::{
    contract_api::{account, runtime, system, Error as ApiError},
    unwrap_or_revert::UnwrapOrRevert,
    value::{account::PurseId, U512},
};

enum Arg {
    TargetPurse = 0,
    Amount = 1,
}

#[no_mangle]
pub extern "C" fn call() {
    let target_purse_id: PurseId = runtime::get_arg(Arg::TargetPurse as u32)
        .unwrap_or_revert_with(ApiError::MissingArgument)
        .unwrap_or_revert_with(ApiError::InvalidArgument);
    let amount: U512 = runtime::get_arg(Arg::Amount as u32)
        .unwrap_or_revert_with(ApiError::MissingArgument)
        .unwrap_or_revert_with(ApiError::InvalidArgument);

    let source_purse_id = account::get_main_purse();

    system::transfer_from_purse_to_purse(source_purse_id, target_purse_id, amount)
        .unwrap_or_revert();
}
