use blake2::{
    digest::{Input, VariableOutput},
    VarBlake2b,
};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;

use contract_ffi::execution::Phase;

use crate::{Address, ADDRESS_LENGTH};

const SEED_LENGTH: usize = 32;

/// An [`AddressGenerator`] generates [`URef`] addresses
pub struct AddressGenerator(ChaChaRng);

impl AddressGenerator {
    /// Creates an [`AddressGenerator`] from a 32-byte hash digest and [`Phase`].
    pub fn new(hash: [u8; 32], phase: Phase) -> AddressGenerator {
        AddressGeneratorBuilder::new()
            .seed_with(&hash)
            .seed_with(&[phase as u8])
            .build()
    }

    pub fn create_address(&mut self) -> Address {
        let mut buff = [0u8; ADDRESS_LENGTH];
        self.0.fill_bytes(&mut buff);
        buff
    }
}

/// A builder for [`AddressGenerator`].
#[derive(Default)]
pub struct AddressGeneratorBuilder {
    data: Vec<u8>,
}

impl AddressGeneratorBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn seed_with(mut self, bytes: &[u8]) -> Self {
        self.data.extend(bytes);
        self
    }

    pub fn build(self) -> AddressGenerator {
        let mut seed: [u8; SEED_LENGTH] = [0u8; SEED_LENGTH];
        let mut hasher = VarBlake2b::new(SEED_LENGTH).unwrap();
        hasher.input(self.data);
        hasher.variable_result(|hash| seed.clone_from_slice(hash));
        AddressGenerator(ChaChaRng::from_seed(seed))
    }
}

#[cfg(test)]
mod tests {
    use contract_ffi::execution::Phase;

    use super::AddressGenerator;

    const DEPLOY_HASH_1: [u8; 32] = [1u8; 32];
    const DEPLOY_HASH_2: [u8; 32] = [2u8; 32];

    #[test]
    fn should_generate_different_numbers_for_different_seeds() {
        let mut ag_a = AddressGenerator::new(DEPLOY_HASH_1, Phase::Session);
        let mut ag_b = AddressGenerator::new(DEPLOY_HASH_2, Phase::Session);
        let random_a = ag_a.create_address();
        let random_b = ag_b.create_address();

        assert_ne!(random_a, random_b)
    }

    #[test]
    fn should_generate_same_numbers_for_same_seed() {
        let mut ag_a = AddressGenerator::new(DEPLOY_HASH_1, Phase::Session);
        let mut ag_b = AddressGenerator::new(DEPLOY_HASH_1, Phase::Session);
        let random_a = ag_a.create_address();
        let random_b = ag_b.create_address();

        assert_eq!(random_a, random_b)
    }

    #[test]
    fn should_not_generate_same_numbers_for_different_phase() {
        let mut ag_a = AddressGenerator::new(DEPLOY_HASH_1, Phase::Payment);
        let mut ag_b = AddressGenerator::new(DEPLOY_HASH_1, Phase::Session);
        let mut ag_c = AddressGenerator::new(DEPLOY_HASH_1, Phase::FinalizePayment);
        let random_a = ag_a.create_address();
        let random_b = ag_b.create_address();
        let random_c = ag_c.create_address();

        assert_ne!(
            random_a, random_b,
            "different phase should have different output"
        );

        assert_ne!(
            random_a, random_c,
            "different phase should have different output"
        );
    }
}
