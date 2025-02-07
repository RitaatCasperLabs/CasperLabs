#![no_std]

extern crate alloc;

use alloc::string::String;

use contract_ffi::{
    contract_api::{storage, Error},
    unwrap_or_revert::UnwrapOrRevert,
};

pub const ENTRY_FUNCTION_NAME: &str = "delegate";
pub const CONTRACT_NAME: &str = "local_state_stored";
pub const SNIPPET: &str = " I've been upgraded!";

#[repr(u16)]
enum CustomError {
    UnableToReadMutatedLocalKey = 0,
    LocalKeyReadMutatedBytesRepr = 1,
}

#[no_mangle]
pub extern "C" fn delegate() {
    local_state::delegate();
    // read from local state
    let mut res: String = storage::read_local(local_state::LOCAL_KEY)
        .unwrap_or_default()
        .unwrap_or_default();

    res.push_str(SNIPPET);
    // Write "Hello, "
    storage::write_local(local_state::LOCAL_KEY, res);

    // Read back
    let res: String = storage::read_local(local_state::LOCAL_KEY)
        .unwrap_or_revert_with(Error::User(CustomError::UnableToReadMutatedLocalKey as u16))
        .unwrap_or_revert_with(Error::User(
            CustomError::LocalKeyReadMutatedBytesRepr as u16,
        ));

    // local state should be available after upgrade
    assert!(
        !res.is_empty(),
        "local value should be accessible post upgrade"
    )
}

#[cfg(not(feature = "lib"))]
#[no_mangle]
pub extern "C" fn call() {
    let key = storage::store_function(ENTRY_FUNCTION_NAME, Default::default())
        .into_turef()
        .unwrap_or_revert_with(Error::UnexpectedContractRefVariant)
        .into();

    contract_ffi::contract_api::runtime::put_key(CONTRACT_NAME, &key);
}
