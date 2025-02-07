#![no_std]

use contract_ffi::{
    contract_api::{account, runtime, Error},
    unwrap_or_revert::UnwrapOrRevert,
    value::account::{ActionType, AddKeyFailure, PublicKey, Weight},
};

#[no_mangle]
pub extern "C" fn call() {
    match account::add_associated_key(PublicKey::new([123; 32]), Weight::new(100)) {
        Err(AddKeyFailure::DuplicateKey) => {}
        Err(_) => runtime::revert(Error::User(50)),
        Ok(_) => {}
    };

    let key_management_threshold: Weight = runtime::get_arg(0)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let deploy_threshold: Weight = runtime::get_arg(1)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);

    if key_management_threshold != Weight::new(0) {
        account::set_action_threshold(ActionType::KeyManagement, key_management_threshold)
            .unwrap_or_revert()
    }

    if deploy_threshold != Weight::new(0) {
        account::set_action_threshold(ActionType::Deployment, deploy_threshold).unwrap_or_revert()
    }
}
