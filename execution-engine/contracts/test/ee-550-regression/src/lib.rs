#![no_std]

extern crate alloc;

use alloc::string::String;

use contract_ffi::{
    contract_api::{account, runtime, Error as ApiError},
    unwrap_or_revert::UnwrapOrRevert,
    value::account::{ActionType, PublicKey, Weight},
};

enum Arg {
    Pass = 0,
}

#[repr(u16)]
enum Error {
    AddKey1 = 0,
    AddKey2 = 1,
    SetActionThreshold = 2,
    RemoveKey = 3,
    UpdateKey = 4,
    UnknownPass = 5,
}

impl Into<ApiError> for Error {
    fn into(self) -> ApiError {
        ApiError::User(self as u16)
    }
}

const KEY_1_ADDR: [u8; 32] = [100; 32];
const KEY_2_ADDR: [u8; 32] = [101; 32];

#[no_mangle]
pub extern "C" fn call() {
    let pass: String = runtime::get_arg(Arg::Pass as u32)
        .unwrap_or_revert_with(ApiError::MissingArgument)
        .unwrap_or_revert_with(ApiError::InvalidArgument);
    match pass.as_str() {
        "init_remove" => {
            account::add_associated_key(PublicKey::new(KEY_1_ADDR), Weight::new(2))
                .unwrap_or_revert_with(Error::AddKey1);
            account::add_associated_key(PublicKey::new(KEY_2_ADDR), Weight::new(255))
                .unwrap_or_revert_with(Error::AddKey2);
            account::set_action_threshold(ActionType::KeyManagement, Weight::new(254))
                .unwrap_or_revert_with(Error::SetActionThreshold);
        }
        "test_remove" => {
            // Deployed with two keys of weights 2 and 255 (total saturates at 255) to satisfy new
            // threshold
            account::remove_associated_key(PublicKey::new(KEY_1_ADDR))
                .unwrap_or_revert_with(Error::RemoveKey);
        }

        "init_update" => {
            account::add_associated_key(PublicKey::new(KEY_1_ADDR), Weight::new(3))
                .unwrap_or_revert_with(Error::AddKey1);
            account::add_associated_key(PublicKey::new(KEY_2_ADDR), Weight::new(255))
                .unwrap_or_revert_with(Error::AddKey2);
            account::set_action_threshold(ActionType::KeyManagement, Weight::new(254))
                .unwrap_or_revert_with(Error::SetActionThreshold);
        }
        "test_update" => {
            // Deployed with two keys of weights 3 and 255 (total saturates at 255) to satisfy new
            // threshold
            account::update_associated_key(PublicKey::new(KEY_1_ADDR), Weight::new(1))
                .unwrap_or_revert_with(Error::UpdateKey);
        }
        _ => {
            runtime::revert(Error::UnknownPass);
        }
    }
}
