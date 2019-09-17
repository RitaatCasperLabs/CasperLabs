use std::collections::HashMap;

use crate::support::test_support::{
    InMemoryWasmTestBuilder, DEFAULT_BLOCK_TIME, STANDARD_PAYMENT_CONTRACT,
};
use contract_ffi::value::U512;
use engine_core::engine_state::MAX_PAYMENT;

const GENESIS_ADDR: [u8; 32] = [7u8; 32];

#[ignore]
#[test]
fn should_not_fail_deserializing() {
    let result = InMemoryWasmTestBuilder::default()
        .run_genesis(GENESIS_ADDR, HashMap::new())
        .exec_with_args(
            GENESIS_ADDR,
            STANDARD_PAYMENT_CONTRACT,
            (U512::from(MAX_PAYMENT),),
            "deserialize_error.wasm",
            (GENESIS_ADDR,),
            DEFAULT_BLOCK_TIME,
            [1u8; 32],
        )
        .commit()
        .finish();

    assert!(result.builder().is_error());
}
