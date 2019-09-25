#![no_std]

extern crate contract_ffi;
use contract_ffi::contract_api::{
    call_contract, create_purse, get_arg, get_pos, main_purse, revert,
    transfer_from_purse_to_purse, PurseTransferResult,
};
use contract_ffi::key::Key;
use contract_ffi::value::uint::U512;

#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = get_pos();
    let source_purse = main_purse();
    let bonding_purse = create_purse();
    let bond_amount: U512 = U512::from(get_arg::<u32>(0).unwrap().unwrap());

    match transfer_from_purse_to_purse(source_purse, bonding_purse, bond_amount) {
        PurseTransferResult::TransferSuccessful => {
            let _result: () = call_contract(
                pos_pointer,
                &("bond", bond_amount, bonding_purse),
            );
        }

        PurseTransferResult::TransferError => revert(1324),
    }
}
