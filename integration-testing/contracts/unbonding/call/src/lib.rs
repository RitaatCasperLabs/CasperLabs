#![no_std]

extern crate contract_ffi;

use contract_ffi::contract_api;
use contract_ffi::value::U512;

#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = contract_api::get_pos();
    // I dont have any safe method to check for the existence of the args.
    // I am utilizing 0(invalid) amount to indicate no args to EE.
    let unbond_amount: Option<U512> = match contract_api::get_arg::<u32>(0).unwrap().unwrap() {
        0 => None,
        amount => Some(amount.into()),
    };
    let _result: () = contract_api::call_contract(pos_pointer, &("unbond", unbond_amount));
}
