#![no_std]

extern crate alloc;

#[cfg(not(feature = "lib"))]
use alloc::collections::BTreeMap;
use alloc::{
    string::{String, ToString},
    vec,
};

use contract_ffi::{
    contract_api::{runtime, storage, system, Error},
    unwrap_or_revert::UnwrapOrRevert,
};

const ENTRY_FUNCTION_NAME: &str = "apply_method";
const CONTRACT_NAME: &str = "purse_holder_stored";
pub const METHOD_ADD: &str = "add";
pub const METHOD_VERSION: &str = "version";
pub const VERSION: &str = "1.0.0";

#[repr(u16)]
enum Args {
    MethodName = 0,
    PurseName = 1,
}

#[repr(u16)]
enum CustomError {
    MissingMethodNameArg = 0,
    InvalidMethodNameArg = 1,
    MissingPurseNameArg = 2,
    InvalidPurseNameArg = 3,
    UnknownMethodName = 4,
}

fn purse_name() -> String {
    runtime::get_arg(Args::PurseName as u32)
        .unwrap_or_revert_with(Error::User(CustomError::MissingPurseNameArg as u16))
        .unwrap_or_revert_with(Error::User(CustomError::InvalidPurseNameArg as u16))
}

#[no_mangle]
pub extern "C" fn apply_method() {
    let method_name: String = runtime::get_arg(Args::MethodName as u32)
        .unwrap_or_revert_with(Error::User(CustomError::MissingMethodNameArg as u16))
        .unwrap_or_revert_with(Error::User(CustomError::InvalidMethodNameArg as u16));
    match method_name.as_str() {
        METHOD_ADD => {
            let purse_name = purse_name();
            let purse_id = system::create_purse();
            runtime::put_key(&purse_name, &purse_id.value().into());
        }
        METHOD_VERSION => runtime::ret(VERSION.to_string(), vec![]),
        _ => runtime::revert(Error::User(CustomError::UnknownMethodName as u16)),
    }
}

#[cfg(not(feature = "lib"))]
#[no_mangle]
pub extern "C" fn call() {
    let key = storage::store_function(ENTRY_FUNCTION_NAME, BTreeMap::new())
        .into_turef()
        .unwrap_or_revert_with(Error::UnexpectedContractRefVariant)
        .into();

    runtime::put_key(CONTRACT_NAME, &key);

    // set version
    let version_key = storage::new_turef(VERSION.to_string()).into();
    runtime::put_key(METHOD_VERSION, &version_key);
}
