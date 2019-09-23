use crate::support::exec_with_return;
use crate::support::test_support::{WasmTestBuilder, DEFAULT_BLOCK_TIME};
use contract_ffi::key::Key;
use contract_ffi::uref::URef;
use contract_ffi::value::Value;
use engine_shared::transform::Transform;
use std::collections::HashMap;
use std::convert::TryInto;

const GENESIS_ADDR: [u8; 32] = [7u8; 32];
const DEPLOY_HASH_1: [u8; 32] = [1u8; 32];

#[ignore]
#[test]
fn should_run_mint_install_contract() {
    let mut builder = WasmTestBuilder::default();

    builder.run_genesis(GENESIS_ADDR, HashMap::new());

    let (ret_value, ret_urefs, effect): (Value, _, _) = exec_with_return::exec(
        &mut builder,
        GENESIS_ADDR,
        "mint_install.wasm",
        DEFAULT_BLOCK_TIME,
        DEPLOY_HASH_1,
        (),
        vec![],
    )
    .expect("should run successfully");

    let ret_value: URef = ret_value.try_into().expect("should contain URef");

    // should return a uref
    assert_eq!(
        ret_value,
        ret_urefs.get(0).cloned().expect("should contain uref")
    );

    // should have written a contract under that uref
    match effect
        .transforms
        .get(&Key::URef(ret_value.remove_access_rights()))
    {
        Some(Transform::Write(Value::Contract(_))) => (),

        _ => panic!("Expected contract to be written under the key"),
    }
}
