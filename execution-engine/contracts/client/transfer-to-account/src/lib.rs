#![no_std]

use contract_ffi::{
    contract_api::{runtime, system, Error},
    unwrap_or_revert::UnwrapOrRevert,
    value::{account::PublicKey, U512},
};

/// Executes mote transfer to supplied public key.
/// Transfers the requested amount.
#[no_mangle]
pub extern "C" fn call() {
    let public_key: PublicKey = runtime::get_arg(0)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let transfer_amount: u64 = runtime::get_arg(1)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let u512_motes = U512::from(transfer_amount);
    system::transfer_to_account(public_key, u512_motes).unwrap_or_revert();
}
