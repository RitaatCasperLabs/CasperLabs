use proptest::{collection::vec, option, prelude::*};

use contract_ffi::{
    gens::{key_arb, value_arb},
    key::Key,
    value::Value,
};
use engine_shared::newtypes::Blake2bHash;

use super::{Pointer, PointerBlock, Trie};

pub fn blake2b_hash_arb() -> impl Strategy<Value = Blake2bHash> {
    vec(any::<u8>(), 0..1000).prop_map(|b| Blake2bHash::new(&b))
}

pub fn trie_pointer_arb() -> impl Strategy<Value = Pointer> {
    prop_oneof![
        blake2b_hash_arb().prop_map(Pointer::LeafPointer),
        blake2b_hash_arb().prop_map(Pointer::NodePointer)
    ]
}

pub fn trie_pointer_block_arb() -> impl Strategy<Value = PointerBlock> {
    (vec(option::of(trie_pointer_arb()), 256).prop_map(|vec| {
        let mut ret: [Option<Pointer>; 256] = [Default::default(); 256];
        ret.clone_from_slice(vec.as_slice());
        ret.into()
    }))
}

pub fn trie_arb() -> impl Strategy<Value = Trie<Key, Value>> {
    prop_oneof![
        (key_arb(), value_arb()).prop_map(|(key, value)| Trie::Leaf { key, value }),
        trie_pointer_block_arb().prop_map(|pointer_block| Trie::Node {
            pointer_block: Box::new(pointer_block)
        }),
        (vec(any::<u8>(), 0..32), trie_pointer_arb())
            .prop_map(|(affix, pointer)| Trie::Extension { affix, pointer })
    ]
}
