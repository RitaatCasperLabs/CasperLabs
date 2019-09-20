#![no_std]
#![feature(cell_update)]

extern crate alloc;
extern crate contract_ffi;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use contract_ffi::contract_api;
use contract_ffi::contract_api::pointers::{ContractPointer, TURef};
use contract_ffi::key::Key;
use contract_ffi::uref::{AccessRights, URef};

const CONTRACT_POINTER: u32 = 0;

enum Error {
    GetArg = 100,
    MissingArg = 101,
    InvalidArgument = 102,
    CreateTURef = 200,
}

const REPLACEMENT_DATA: &str = "bawitdaba";

#[no_mangle]
pub extern "C" fn call() {
    let contract_pointer: ContractPointer = contract_api::get_arg::<Key>(CONTRACT_POINTER)
        .unwrap_or_else(|| contract_api::revert(Error::MissingArg as u32))
        .unwrap_or_else(|_| contract_api::revert(Error::InvalidArgument as u32))
        .to_c_ptr()
        .unwrap_or_else(|| contract_api::revert(Error::GetArg as u32));

    let reference: URef = contract_api::call_contract(contract_pointer, &(), &Vec::new());

    let forged_reference: TURef<String> = {
        let ret = URef::new(reference.addr(), AccessRights::READ_ADD_WRITE);
        TURef::from_uref(ret).unwrap_or_else(|_| contract_api::revert(Error::CreateTURef as u32))
    };

    contract_api::write(forged_reference, REPLACEMENT_DATA.to_string())
}
