#![no_std]

extern crate contract_ffi;

use contract_ffi::contract_api::pointers::ContractPointer;
use contract_ffi::contract_api::{self, PurseTransferResult};
use contract_ffi::key::Key;
use contract_ffi::value::account::{PublicKey, PurseId};
use contract_ffi::value::U512;

fn set_refund_purse(pos: &ContractPointer, p: &PurseId) {
    contract_api::call_contract::<_, ()>(
        pos.clone(),
        &("set_refund_purse", *p),
    );
}

fn get_payment_purse(pos: &ContractPointer) -> PurseId {
    contract_api::call_contract(pos.clone(), &("get_payment_purse",))
}

fn submit_payment(pos: &ContractPointer, amount: U512) {
    let payment_purse = get_payment_purse(pos);
    let main_purse = contract_api::main_purse();
    if let PurseTransferResult::TransferError =
        contract_api::transfer_from_purse_to_purse(main_purse, payment_purse, amount)
    {
        contract_api::revert(99);
    }
}

fn finalize_payment(pos: &ContractPointer, amount_spent: U512, account: PublicKey) {
    contract_api::call_contract::<_, ()>(
        pos.clone(),
        &("finalize_payment", amount_spent, account),
    )
}

#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = contract_api::get_pos();

    let payment_amount: U512 = contract_api::get_arg(0).unwrap().unwrap();
    let refund_purse_flag: u8 = contract_api::get_arg(1).unwrap().unwrap();
    let amount_spent: U512 = contract_api::get_arg(2).unwrap().unwrap();
    let account: PublicKey = contract_api::get_arg(3).unwrap().unwrap();

    submit_payment(&pos_pointer, payment_amount);
    if refund_purse_flag != 0 {
        let refund_purse = contract_api::create_purse();
        contract_api::add_uref("local_refund_purse", &Key::URef(refund_purse.value()));
        set_refund_purse(&pos_pointer, &refund_purse);
    }
    finalize_payment(&pos_pointer, amount_spent, account);
}
