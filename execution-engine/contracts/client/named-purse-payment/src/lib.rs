#![no_std]

extern crate alloc;
extern crate contract_ffi;
use alloc::string::String;
use contract_ffi::contract_api::{self, Error, PurseTransferResult};
use contract_ffi::value::account::PurseId;
use contract_ffi::value::U512;

const GET_PAYMENT_PURSE: &str = "get_payment_purse";
const SET_REFUND_PURSE: &str = "set_refund_purse";

enum Arg {
    PurseName = 0,
    Amount = 1,
}

#[no_mangle]
pub extern "C" fn call() {
    let purse_name: String = match contract_api::get_arg(Arg::PurseName as u32) {
        Some(Ok(data)) => data,
        Some(Err(_)) => contract_api::revert(Error::InvalidArgument.into()),
        None => contract_api::revert(Error::MissingArgument.into()),
    };

    let purse_key = contract_api::get_uref(&purse_name)
        .unwrap_or_else(|| contract_api::revert(Error::InvalidPurseName.into()));
    let purse = match purse_key.as_uref() {
        Some(uref) => PurseId::new(*uref),
        None => contract_api::revert(Error::InvalidPurse.into()),
    };

    let amount: U512 = match contract_api::get_arg(Arg::Amount as u32) {
        Some(Ok(data)) => data,
        Some(Err(_)) => contract_api::revert(Error::InvalidArgument.into()),
        None => contract_api::revert(Error::MissingArgument.into()),
    };

    let pos_pointer = contract_api::get_pos();

    let payment_purse: PurseId =
        contract_api::call_contract(pos_pointer.clone(), &(GET_PAYMENT_PURSE,));

    contract_api::call_contract::<_, ()>(pos_pointer, &(SET_REFUND_PURSE, purse));

    if let PurseTransferResult::TransferError =
        contract_api::transfer_from_purse_to_purse(purse, payment_purse, amount)
    {
        contract_api::revert(Error::Transfer.into());
    }
}
