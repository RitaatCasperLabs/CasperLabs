#![no_std]

extern crate alloc;

use alloc::vec;

use contract_ffi::{
    contract_api::{account, runtime, system, Error},
    key::Key,
    unwrap_or_revert::UnwrapOrRevert,
    value::uint::U512,
};

const BOND_METHOD_NAME: &str = "bond";

// Bonding contract.
//
// Accepts bonding amount (of type `u64`) as first argument.
// Issues bonding request to the PoS contract.
#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = system::get_proof_of_stake();
    let source_purse = account::get_main_purse();
    let bonding_purse = system::create_purse();
    let bond_amount: U512 = runtime::get_arg::<u64>(0)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument)
        .into();

    system::transfer_from_purse_to_purse(source_purse, bonding_purse, bond_amount)
        .unwrap_or_revert();
    runtime::call_contract(
        pos_pointer,
        &(BOND_METHOD_NAME, bond_amount, bonding_purse),
        &vec![Key::URef(bonding_purse.value())],
    )
}
