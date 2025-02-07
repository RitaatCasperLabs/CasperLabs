#![no_std]

extern crate alloc;

use alloc::vec;

use contract_ffi::{
    contract_api::{runtime, system, Error},
    unwrap_or_revert::UnwrapOrRevert,
    value::uint::U512,
};

const UNBOND_METHOD_NAME: &str = "unbond";

// Unbonding contract.
//
// Accepts unbonding amount (of type `Option<u64>`) as first argument.
// Unbonding with `None` unbonds all stakes in the PoS contract.
// Otherwise (`Some<u64>`) unbonds with part of the bonded stakes.
#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = system::get_proof_of_stake();

    let arg_0: Option<u64> = runtime::get_arg(0)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let unbond_amount: Option<U512> = arg_0.map(Into::into);

    runtime::call_contract(pos_pointer, &(UNBOND_METHOD_NAME, unbond_amount), &vec![])
}
