#![no_std]

extern crate contract_ffi;

use contract_ffi::contract_api;
use contract_ffi::value::uint::U512;
use contract_ffi::value::Value;

const UNBOND_METHOD_NAME: &str = "unbond";

// Unbonding contract.
//
// Accepts unbonding amount (of type `Option<u64>`) as first argument.
// Unbonding with `None` unbonds all stakes in the PoS contract.
// Otherwise (`Some<u64>`) unbonds with part of the bonded stakes.
#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = unwrap_or_revert(contract_api::get_pos(), 77);

    let unbound_amount_value: Value = contract_api::get_arg(0);
    let unbond_amount: Option<u64> = unbound_amount_value.try_deserialize().unwrap();
    let unbond_amount: Option<U512> = unbond_amount.map(U512::from);

    contract_api::call_contract(
        pos_pointer,
        &(
            UNBOND_METHOD_NAME,
            Value::from_serializable(unbond_amount).unwrap(),
        ),
    )
}

fn unwrap_or_revert<T>(option: Option<T>, code: u32) -> T {
    if let Some(value) = option {
        value
    } else {
        contract_api::revert(code)
    }
}
