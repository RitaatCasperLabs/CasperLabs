#![no_std]
#![feature(cell_update)]

extern crate alloc;
extern crate contract_ffi;
use alloc::string::{String, ToString};
use contract_ffi::contract_api;

pub const LOCAL_KEY: [u8; 32] = [66u8; 32];
pub const HELLO_PREFIX: &str = " Hello, ";
pub const WORLD_SUFFIX: &str = "world!";

#[repr(u16)]
enum CustomError {
    UnableToReadMutatedLocalKey = 100,
    LocalKeyReadMutatedBytesRepr = 101,
}

pub fn delegate() {
    // Appends " Hello, world!" to a [66; 32] local key with spaces trimmed.
    // Two runs should yield value "Hello, world! Hello, world!"
    // read from local state
    let mut res: String = contract_api::read_local(LOCAL_KEY)
        .unwrap_or_default()
        .unwrap_or_default();

    res.push_str(HELLO_PREFIX);
    // Write "Hello, "
    contract_api::write_local(LOCAL_KEY, res);

    // Read (this should exercise cache)
    let mut res: String = contract_api::read_local(LOCAL_KEY)
        .unwrap_or_else(|_| {
            contract_api::revert(
                contract_api::Error::User(CustomError::UnableToReadMutatedLocalKey as u16).into(),
            )
        })
        .unwrap_or_else(|| {
            contract_api::revert(
                contract_api::Error::User(CustomError::LocalKeyReadMutatedBytesRepr as u16).into(),
            )
        });
    // Append
    res.push_str(WORLD_SUFFIX);
    // Write
    contract_api::write_local(LOCAL_KEY, res.trim().to_string());
}

#[cfg(not(feature = "lib"))]
#[no_mangle]
pub extern "C" fn call() {
    delegate()
}
