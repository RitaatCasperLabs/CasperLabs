#![no_std]

use contract_ffi::{
    contract_api::{runtime, storage},
    key::Key,
};

#[no_mangle]
pub extern "C" fn call() {
    let res1 = runtime::get_key("nonexistinguref");
    assert!(res1.is_none());

    let key = Key::URef(storage::new_turef(()).into());
    runtime::put_key("nonexistinguref", &key);

    let res2 = runtime::get_key("nonexistinguref");

    assert_eq!(res2, Some(key));
}
