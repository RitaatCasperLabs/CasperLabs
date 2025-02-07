#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;

use contract_ffi::{
    contract_api::{runtime, storage},
    value::account::PublicKey,
};

const GET_CALLER_EXT: &str = "get_caller_ext";
const GET_CALLER_KEY: &str = "get_caller";

fn test_get_caller() {
    // Assumes that will be called using test framework genesis account with
    // public key == 'ae7cd84d61ff556806691be61e6ab217791905677adbbe085b8c540d916e8393'
    // Will fail if we ever change that.
    let caller = runtime::get_caller();
    let expected_caller = PublicKey::new([
        174, 124, 216, 77, 97, 255, 85, 104, 6, 105, 27, 230, 30, 106, 178, 23, 121, 25, 5, 103,
        122, 219, 190, 8, 91, 140, 84, 13, 145, 110, 131, 147,
    ]);
    assert_eq!(caller, expected_caller);
}

#[no_mangle]
pub extern "C" fn get_caller_ext() {
    // works in sub-calls
    test_get_caller();
}

#[no_mangle]
pub extern "C" fn call() {
    // works in session code
    test_get_caller();
    let pointer = storage::store_function_at_hash(GET_CALLER_EXT, BTreeMap::new());
    runtime::put_key(GET_CALLER_KEY, &pointer.into());
}
