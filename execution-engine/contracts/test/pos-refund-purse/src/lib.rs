#![no_std]

extern crate contract_ffi;

use contract_ffi::contract_api::pointers::ContractPointer;
use contract_ffi::contract_api::{self, Error as ApiError, PurseTransferResult};
use contract_ffi::value::account::PurseId;
use contract_ffi::value::U512;

#[repr(u16)]
enum Error {
    ShouldNotExist = 0,
    NotFound,
    Invalid,
    IncorrectAccessRights,
}

fn set_refund_purse(pos: &ContractPointer, p: &PurseId) {
    contract_api::call_contract::<_, ()>(pos.clone(), &("set_refund_purse", *p));
}

fn get_refund_purse(pos: &ContractPointer) -> Option<PurseId> {
    // TODO(mpapierski): Identify additional Value variants
    contract_api::call_contract(pos.clone(), &("get_refund_purse",))
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
        contract_api::revert(ApiError::Transfer.into());
    }
}

#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = contract_api::get_pos();

    let p1 = contract_api::create_purse();
    let p2 = contract_api::create_purse();

    // get_refund_purse should return None before setting it
    let refund_result = get_refund_purse(&pos_pointer);
    if refund_result.is_some() {
        contract_api::revert(ApiError::User(Error::ShouldNotExist as u16).into());
    }

    // it should return Some(x) after calling set_refund_purse(x)
    set_refund_purse(&pos_pointer, &p1);
    let refund_purse = match get_refund_purse(&pos_pointer) {
        None => contract_api::revert(ApiError::User(Error::NotFound as u16).into()),
        Some(x) if x.value().addr() == p1.value().addr() => x.value(),
        Some(_) => contract_api::revert(ApiError::User(Error::Invalid as u16).into()),
    };

    // the returned purse should not have any access rights
    if refund_purse.is_addable() || refund_purse.is_writeable() || refund_purse.is_readable() {
        contract_api::revert(ApiError::User(Error::IncorrectAccessRights as u16).into())
    }

    // get_refund_purse should return correct value after setting a second time
    set_refund_purse(&pos_pointer, &p2);
    match get_refund_purse(&pos_pointer) {
        None => contract_api::revert(ApiError::User(Error::NotFound as u16).into()),
        Some(x) if x.value().addr() == p2.value().addr() => (),
        Some(_) => contract_api::revert(ApiError::User(Error::Invalid as u16).into()),
    }

    let payment_amount: U512 = match contract_api::get_arg(0) {
        Some(Ok(data)) => data,
        Some(Err(_)) => contract_api::revert(ApiError::InvalidArgument.into()),
        None => contract_api::revert(ApiError::MissingArgument.into()),
    };

    submit_payment(&pos_pointer, payment_amount);
}
