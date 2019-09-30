#![no_std]
#![feature(cell_update)]

extern crate alloc;
extern crate contract_ffi;

mod capabilities;

// These types are purposely defined in a separate module
// so that their constructors are hidden and therefore
// we must use the conversion methods from Key elsewhere
// in the code.
mod internal_purse_id;

mod mint;

use alloc::string::String;
use core::convert::TryInto;

use contract_ffi::contract_api::{self, Error as ApiError};
use contract_ffi::key::Key;
use contract_ffi::system_contracts::mint::error::Error;
use contract_ffi::uref::{AccessRights, URef};
use contract_ffi::value::account::KEY_SIZE;
use contract_ffi::value::{Value, U512};

use capabilities::{ARef, RAWRef};
use internal_purse_id::{DepositId, WithdrawId};
use mint::Mint;

const SYSTEM_ACCOUNT: [u8; KEY_SIZE] = [0u8; KEY_SIZE];

struct CLMint;

impl Mint<ARef<U512>, RAWRef<U512>> for CLMint {
    type PurseId = WithdrawId;
    type DepOnlyId = DepositId;

    fn mint(&self, initial_balance: U512) -> Result<Self::PurseId, Error> {
        let caller = contract_api::get_caller();
        if !initial_balance.is_zero() && caller.value() != SYSTEM_ACCOUNT {
            return Err(Error::InvalidNonEmptyPurseCreation);
        }

        let balance_uref: Key = contract_api::new_turef(initial_balance).into();

        let purse_key: URef = contract_api::new_turef(()).into();
        let purse_uref_name = purse_key.remove_access_rights().as_string();

        let purse_id: WithdrawId = WithdrawId::from_uref(purse_key).unwrap();

        // store balance uref so that the runtime knows the mint has full access
        contract_api::add_uref(&purse_uref_name, &balance_uref);

        // store association between purse id and balance uref
        //
        // Gorski writes:
        //   I'm worried that this can lead to overwriting of values in the local state.
        //   Since it accepts a raw byte array it's possible to construct one by hand.
        // Of course,   a key can be overwritten only when that write is
        // performed in the "owner" context   so it aligns with other semantics
        // of write but I would prefer if were able to enforce   uniqueness
        // somehow.
        contract_api::write_local(purse_id.raw_id(), balance_uref);

        Ok(purse_id)
    }

    fn lookup(&self, p: Self::PurseId) -> Option<RAWRef<U512>> {
        contract_api::read_local(p.raw_id())
            .ok()?
            .and_then(|key: Key| key.try_into().ok())
    }

    fn dep_lookup(&self, p: Self::DepOnlyId) -> Option<ARef<U512>> {
        contract_api::read_local(p.raw_id())
            .ok()?
            .and_then(|key: Key| key.try_into().ok())
    }
}

pub fn delegate() {
    let mint = CLMint;
    let method_name: String = match contract_api::get_arg(0) {
        Some(Ok(data)) => data,
        Some(Err(_)) => contract_api::revert(ApiError::InvalidArgument.into()),
        None => contract_api::revert(ApiError::MissingArgument.into()),
    };

    match method_name.as_str() {
        // argument: U512
        // return: URef
        "mint" => {
            let amount: U512 = match contract_api::get_arg(1) {
                Some(Ok(data)) => data,
                Some(Err(_)) => contract_api::revert(ApiError::InvalidArgument.into()),
                None => contract_api::revert(ApiError::MissingArgument.into()),
            };

            let purse_uref = mint
                .mint(amount)
                .map(|purse_id| URef::new(purse_id.raw_id(), AccessRights::READ_ADD_WRITE))
                // NOTE: This unwraps mint's error at the call site because callers expects a single
                // Value for URef verification purposes. If it transferred as
                // Value::ByteArray then the URef can't be inferred from the return value, and it
                // can raise unexpected ForgedReference errors.
                .unwrap_or_else(|e| contract_api::revert(e as u32));

            contract_api::ret(purse_uref)
        }

        "create" => {
            let purse_id = mint.create();
            let purse_key = URef::new(purse_id.raw_id(), AccessRights::READ_ADD_WRITE);
            contract_api::ret(purse_key)
        }

        "balance" => {
            let key: URef = match contract_api::get_arg(1) {
                Some(Ok(data)) => data,
                Some(Err(_)) => contract_api::revert(ApiError::InvalidArgument.into()),
                None => contract_api::revert(ApiError::MissingArgument.into()),
            };
            let purse_id: WithdrawId = WithdrawId::from_uref(key).unwrap();
            let balance_uref = mint.lookup(purse_id);
            let balance: Option<U512> =
                balance_uref.and_then(|uref| contract_api::read(uref.into()).unwrap_or_default());
            contract_api::ret(Value::from_serializable(balance).unwrap());
        }

        "transfer" => {
            let source: URef = match contract_api::get_arg(1) {
                Some(Ok(data)) => data,
                Some(Err(_)) => contract_api::revert(ApiError::InvalidArgument.into()),
                None => contract_api::revert(ApiError::MissingArgument.into()),
            };
            let target: URef = match contract_api::get_arg(2) {
                Some(Ok(data)) => data,
                Some(Err(_)) => contract_api::revert(ApiError::InvalidArgument.into()),
                None => contract_api::revert(ApiError::MissingArgument.into()),
            };
            let amount: U512 = match contract_api::get_arg(3) {
                Some(Ok(data)) => data,
                Some(Err(_)) => contract_api::revert(ApiError::InvalidArgument.into()),
                None => contract_api::revert(ApiError::MissingArgument.into()),
            };

            let source: WithdrawId = match WithdrawId::from_uref(source) {
                Ok(withdraw_id) => withdraw_id,
                Err(error) => {
                    let transfer_result: Result<(), Error> = Err(error.into());
                    // TODO(mpapierski): Identify additional Value variants
                    contract_api::ret(Value::from_serializable(transfer_result).unwrap());
                }
            };

            let target: DepositId = match DepositId::from_uref(target) {
                Ok(deposit_id) => deposit_id,
                Err(error) => {
                    let transfer_result: Result<(), Error> = Err(error.into());
                    // TODO(mpapierski): Identify additional Value variants
                    contract_api::ret(Value::from_serializable(transfer_result).unwrap());
                }
            };

            let transfer_result = mint.transfer(source, target, amount);
            contract_api::ret(Value::from_serializable(transfer_result).unwrap());
        }

        _ => panic!("Unknown method name!"),
    }
}

#[cfg(not(feature = "lib"))]
#[no_mangle]
pub extern "C" fn call() {
    delegate();
}
