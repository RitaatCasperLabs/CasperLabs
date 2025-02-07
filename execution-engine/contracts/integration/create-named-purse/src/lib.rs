#![no_std]

extern crate alloc;

use alloc::string::String;

use contract_ffi::{
    contract_api::{account, runtime, system, Error},
    key::Key,
    unwrap_or_revert::UnwrapOrRevert,
    value::{account::PurseId, U512},
};

enum Arg {
    Amount = 0,
    Name = 1,
}

#[no_mangle]
pub extern "C" fn call() {
    let amount: U512 = runtime::get_arg(Arg::Amount as u32)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let name: String = runtime::get_arg(Arg::Name as u32)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let main_purse: PurseId = account::get_main_purse();
    let new_purse: PurseId = system::create_purse();

    system::transfer_from_purse_to_purse(main_purse, new_purse, amount).unwrap_or_revert();

    let new_purse_key: Key = new_purse.value().into();
    runtime::put_key(&name, &new_purse_key);
}
