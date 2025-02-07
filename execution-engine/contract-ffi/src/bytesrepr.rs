use super::alloc::{
    collections::{BTreeMap, TryReserveError},
    string::String,
    vec::Vec,
};

use core::mem::{size_of, MaybeUninit};
use failure::Fail;

use crate::value::{ProtocolVersion, SemVer};

pub const I32_SERIALIZED_LENGTH: usize = size_of::<i32>();
pub const U8_SERIALIZED_LENGTH: usize = size_of::<u8>();
pub const U16_SERIALIZED_LENGTH: usize = size_of::<u16>();
pub const U32_SERIALIZED_LENGTH: usize = size_of::<u32>();
pub const U64_SERIALIZED_LENGTH: usize = size_of::<u64>();
pub const U128_SERIALIZED_LENGTH: usize = size_of::<u128>();
pub const U256_SERIALIZED_LENGTH: usize = U128_SERIALIZED_LENGTH * 2;
pub const U512_SERIALIZED_LENGTH: usize = U256_SERIALIZED_LENGTH * 2;
pub const OPTION_TAG_SERIALIZED_LENGTH: usize = 1;
pub const SEM_VER_SERIALIZED_LENGTH: usize = 12;

const N256: usize = 256;

pub trait ToBytes {
    fn to_bytes(&self) -> Result<Vec<u8>, Error>;
}

pub trait FromBytes: Sized {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error>;
}

#[derive(Debug, Fail, PartialEq, Eq, Clone)]
#[repr(u8)]
pub enum Error {
    #[fail(display = "Deserialization error: early end of stream")]
    EarlyEndOfStream = 0,

    #[fail(display = "Deserialization error: formatting error")]
    FormattingError,

    #[fail(display = "Deserialization error: left-over bytes")]
    LeftOverBytes,

    #[fail(display = "Serialization error: out of memory")]
    OutOfMemoryError,
}

impl From<TryReserveError> for Error {
    fn from(_: TryReserveError) -> Error {
        Error::OutOfMemoryError
    }
}

pub fn deserialize<T: FromBytes>(bytes: &[u8]) -> Result<T, Error> {
    let (t, rem): (T, &[u8]) = FromBytes::from_bytes(bytes)?;
    if rem.is_empty() {
        Ok(t)
    } else {
        Err(Error::LeftOverBytes)
    }
}

pub fn serialize(t: impl ToBytes) -> Result<Vec<u8>, Error> {
    t.to_bytes()
}

pub fn safe_split_at(bytes: &[u8], n: usize) -> Result<(&[u8], &[u8]), Error> {
    if n > bytes.len() {
        Err(Error::EarlyEndOfStream)
    } else {
        Ok(bytes.split_at(n))
    }
}

impl ToBytes for u8 {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut result = Vec::with_capacity(1);
        result.push(*self);
        Ok(result)
    }
}

impl FromBytes for u8 {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        match bytes.split_first() {
            None => Err(Error::EarlyEndOfStream),
            Some((byte, rem)) => Ok((*byte, rem)),
        }
    }
}

impl ToBytes for i32 {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl FromBytes for i32 {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let mut result: [u8; I32_SERIALIZED_LENGTH] = [0u8; I32_SERIALIZED_LENGTH];
        let (bytes, rem) = safe_split_at(bytes, I32_SERIALIZED_LENGTH)?;
        result.copy_from_slice(bytes);
        Ok((i32::from_le_bytes(result), rem))
    }
}

impl ToBytes for u32 {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl FromBytes for u32 {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let mut result: [u8; U32_SERIALIZED_LENGTH] = [0u8; U32_SERIALIZED_LENGTH];
        let (bytes, rem) = safe_split_at(bytes, U32_SERIALIZED_LENGTH)?;
        result.copy_from_slice(bytes);
        Ok((u32::from_le_bytes(result), rem))
    }
}

impl ToBytes for u64 {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl FromBytes for u64 {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let mut result: [u8; 8] = [0u8; 8];
        let (bytes, rem) = safe_split_at(bytes, 8)?;
        result.copy_from_slice(bytes);
        Ok((u64::from_le_bytes(result), rem))
    }
}

impl FromBytes for Vec<u8> {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (size, mut stream): (u32, &[u8]) = FromBytes::from_bytes(bytes)?;
        let mut result: Vec<u8> = Vec::new();
        result.try_reserve_exact(size as usize)?;
        for _ in 0..size {
            let (t, rem): (u8, &[u8]) = FromBytes::from_bytes(stream)?;
            result.push(t);
            stream = rem;
        }
        Ok((result, stream))
    }
}

impl ToBytes for Vec<u8> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        // Return error if size of serialized vector would exceed limit for
        // 32-bit architecture.
        if self.len() >= u32::max_value() as usize - U32_SERIALIZED_LENGTH {
            return Err(Error::OutOfMemoryError);
        }
        let size = self.len() as u32;
        let mut result: Vec<u8> = Vec::with_capacity(U32_SERIALIZED_LENGTH + size as usize);
        result.extend(size.to_bytes()?);
        result.extend(self);
        Ok(result)
    }
}

impl FromBytes for Vec<i32> {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (size, mut stream): (u32, &[u8]) = FromBytes::from_bytes(bytes)?;
        let mut result: Vec<i32> = Vec::new();
        result.try_reserve_exact(size as usize)?;
        for _ in 0..size {
            let (t, rem): (i32, &[u8]) = FromBytes::from_bytes(stream)?;
            result.push(t);
            stream = rem;
        }
        Ok((result, stream))
    }
}

impl<T: ToBytes> ToBytes for Option<T> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        match self {
            Some(v) => {
                let mut value = v.to_bytes()?;
                if value.len() >= u32::max_value() as usize - U8_SERIALIZED_LENGTH {
                    return Err(Error::OutOfMemoryError);
                }
                let mut result: Vec<u8> = Vec::with_capacity(U8_SERIALIZED_LENGTH + value.len());
                result.append(&mut 1u8.to_bytes()?);
                result.append(&mut value);
                Ok(result)
            }
            // In the case of None there is no value to serialize, but we still
            // need to write out a tag to indicate which variant we are using
            None => Ok(0u8.to_bytes()?),
        }
    }
}

impl<T: FromBytes> FromBytes for Option<T> {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (tag, rem): (u8, &[u8]) = FromBytes::from_bytes(bytes)?;
        match tag {
            0 => Ok((None, rem)),
            1 => {
                let (t, rem): (T, &[u8]) = FromBytes::from_bytes(rem)?;
                Ok((Some(t), rem))
            }
            _ => Err(Error::FormattingError),
        }
    }
}

impl ToBytes for Vec<i32> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        // Return error if size of vector would exceed length of serialized data
        if self.len() * I32_SERIALIZED_LENGTH >= u32::max_value() as usize - U32_SERIALIZED_LENGTH {
            return Err(Error::OutOfMemoryError);
        }
        let size = self.len() as u32;
        let mut result: Vec<u8> =
            Vec::with_capacity(U32_SERIALIZED_LENGTH + (I32_SERIALIZED_LENGTH * size as usize));
        result.extend(size.to_bytes()?);
        result.extend(
            self.iter()
                .map(ToBytes::to_bytes)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten(),
        );
        Ok(result)
    }
}

impl FromBytes for Vec<Vec<u8>> {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (size, mut stream): (u32, &[u8]) = FromBytes::from_bytes(bytes)?;
        let mut result = Vec::new();
        result.try_reserve_exact(size as usize)?;
        for _ in 0..size {
            let (v, rem): (Vec<u8>, &[u8]) = FromBytes::from_bytes(stream)?;
            result.push(v);
            stream = rem;
        }
        Ok((result, stream))
    }
}

impl ToBytes for Vec<Vec<u8>> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        if self.len() >= u32::max_value() as usize - U32_SERIALIZED_LENGTH {
            // Fail fast for large enough vectors
            return Err(Error::OutOfMemoryError);
        }
        // Calculate total length of all vectors
        let total_length: usize = self.iter().map(Vec::len).sum();
        // It could be either length of vector which serialized would exceed
        // the maximum size of vector (i.e. vector of vectors of size 1),
        // or the total length of all vectors (i.e. vector of size 1 which holds
        // vector of size 2^32-1)
        if total_length >= u32::max_value() as usize - U32_SERIALIZED_LENGTH {
            return Err(Error::OutOfMemoryError);
        }
        let size = self.len() as u32;
        let mut result: Vec<u8> =
            Vec::with_capacity(U32_SERIALIZED_LENGTH + size as usize + total_length);
        result.extend_from_slice(&size.to_bytes()?);
        for n in 0..size {
            result.extend_from_slice(&self[n as usize].to_bytes()?);
        }
        Ok(result)
    }
}

impl FromBytes for Vec<String> {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (size, mut stream): (u32, &[u8]) = FromBytes::from_bytes(bytes)?;
        let mut result = Vec::new();
        result.try_reserve_exact(size as usize)?;
        for _ in 0..size {
            let (s, rem): (String, &[u8]) = FromBytes::from_bytes(stream)?;
            result.push(s);
            stream = rem;
        }
        Ok((result, stream))
    }
}

impl ToBytes for Vec<String> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let size = self.len() as u32;
        let mut result: Vec<u8> = Vec::with_capacity(U32_SERIALIZED_LENGTH);
        result.extend(size.to_bytes()?);
        result.extend(
            self.iter()
                .map(ToBytes::to_bytes)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten(),
        );
        Ok(result)
    }
}

macro_rules! impl_byte_array {
    ($len:expr) => {
        impl ToBytes for [u8; $len] {
            fn to_bytes(&self) -> Result<Vec<u8>, Error> {
                Ok(self.to_vec())
            }
        }

        impl FromBytes for [u8; $len] {
            fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
                let (bytes, rem) = safe_split_at(bytes, $len)?;
                let mut result = [0u8; $len];
                result.copy_from_slice(bytes);
                Ok((result, rem))
            }
        }
    };
}

impl_byte_array!(4);
impl_byte_array!(5);
impl_byte_array!(8);
impl_byte_array!(32);

impl<T: ToBytes> ToBytes for [T; N256] {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        if self.len() * size_of::<T>() >= u32::max_value() as usize - U32_SERIALIZED_LENGTH {
            return Err(Error::OutOfMemoryError);
        }
        let mut result: Vec<u8> =
            Vec::with_capacity(U32_SERIALIZED_LENGTH + (self.len() * size_of::<T>()));
        result.extend((N256 as u32).to_bytes()?);
        result.extend(
            self.iter()
                .map(ToBytes::to_bytes)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten(),
        );
        Ok(result)
    }
}

impl<T: FromBytes> FromBytes for [T; N256] {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (size, mut stream): (u32, &[u8]) = FromBytes::from_bytes(bytes)?;
        if size != N256 as u32 {
            return Err(Error::FormattingError);
        }
        let mut result: MaybeUninit<[T; N256]> = MaybeUninit::uninit();
        let result_ptr = result.as_mut_ptr() as *mut T;
        unsafe {
            for i in 0..N256 {
                let (t, rem): (T, &[u8]) = FromBytes::from_bytes(stream)?;
                result_ptr.add(i).write(t);
                stream = rem;
            }
            Ok((result.assume_init(), stream))
        }
    }
}

impl ToBytes for String {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.as_str().to_bytes()
    }
}

impl FromBytes for String {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (str_bytes, rem): (Vec<u8>, &[u8]) = FromBytes::from_bytes(bytes)?;
        let result = String::from_utf8(str_bytes).map_err(|_| Error::FormattingError)?;
        Ok((result, rem))
    }
}

impl ToBytes for () {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(Vec::new())
    }
}

impl FromBytes for () {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        Ok(((), bytes))
    }
}

impl<K, V> ToBytes for BTreeMap<K, V>
where
    K: ToBytes,
    V: ToBytes,
{
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let num_keys = self.len() as u32;
        let bytes = self
            .iter()
            .map(move |(k, v)| {
                let k_bytes = k.to_bytes().map_err(Error::from);
                let v_bytes = v.to_bytes().map_err(Error::from);
                // For each key and value pair create a vector of
                // serialization results
                let mut vs = Vec::with_capacity(2);
                vs.push(k_bytes);
                vs.push(v_bytes);
                vs
            })
            // Flatten iterable of key and value pairs
            .flatten()
            // Collect into a single Result of bytes (if successful)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten();

        let (lower_bound, _upper_bound) = bytes.size_hint();
        if lower_bound >= u32::max_value() as usize - U32_SERIALIZED_LENGTH {
            return Err(Error::OutOfMemoryError);
        }
        let mut result: Vec<u8> = Vec::with_capacity(U32_SERIALIZED_LENGTH + lower_bound);
        result.append(&mut num_keys.to_bytes()?);
        result.extend(bytes);
        Ok(result)
    }
}

impl<K, V> FromBytes for BTreeMap<K, V>
where
    K: FromBytes + Ord,
    V: FromBytes,
{
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (num_keys, mut stream): (u32, &[u8]) = FromBytes::from_bytes(bytes)?;
        let mut result = BTreeMap::new();
        for _ in 0..num_keys {
            let (k, rem): (K, &[u8]) = FromBytes::from_bytes(stream)?;
            let (v, rem): (V, &[u8]) = FromBytes::from_bytes(rem)?;
            result.insert(k, v);
            stream = rem;
        }
        Ok((result, stream))
    }
}

impl ToBytes for str {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        if self.len() >= u32::max_value() as usize - U32_SERIALIZED_LENGTH {
            return Err(Error::OutOfMemoryError);
        }
        let bytes = self.as_bytes();
        let size = self.len();
        let mut result: Vec<u8> = Vec::with_capacity(U32_SERIALIZED_LENGTH + size);
        result.extend((size as u32).to_bytes()?);
        result.extend(bytes);
        Ok(result)
    }
}

impl ToBytes for &str {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        (*self).to_bytes()
    }
}

impl<T: ToBytes, E: ToBytes> ToBytes for Result<T, E> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let (mut variant, mut value) = match self {
            Ok(result) => (1u8.to_bytes()?, result.to_bytes()?),
            Err(error) => (0u8.to_bytes()?, error.to_bytes()?),
        };
        let mut result: Vec<u8> = Vec::with_capacity(U8_SERIALIZED_LENGTH + value.len());
        result.append(&mut variant);
        result.append(&mut value);
        Ok(result)
    }
}

impl<T: FromBytes, E: FromBytes> FromBytes for Result<T, E> {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (variant, rem): (u8, &[u8]) = FromBytes::from_bytes(bytes)?;
        match variant {
            0 => {
                let (value, rem): (E, &[u8]) = FromBytes::from_bytes(rem)?;
                Ok((Err(value), rem))
            }
            1 => {
                let (value, rem): (T, &[u8]) = FromBytes::from_bytes(rem)?;
                Ok((Ok(value), rem))
            }
            _ => Err(Error::FormattingError),
        }
    }
}

impl ToBytes for SemVer {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut ret: Vec<u8> = Vec::with_capacity(SEM_VER_SERIALIZED_LENGTH);
        ret.append(&mut self.major.to_bytes()?);
        ret.append(&mut self.minor.to_bytes()?);
        ret.append(&mut self.patch.to_bytes()?);
        Ok(ret)
    }
}

impl FromBytes for SemVer {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (major, rem): (u32, &[u8]) = FromBytes::from_bytes(bytes)?;
        let (minor, rem): (u32, &[u8]) = FromBytes::from_bytes(rem)?;
        let (patch, rem): (u32, &[u8]) = FromBytes::from_bytes(rem)?;
        Ok((SemVer::new(major, minor, patch), rem))
    }
}

impl ToBytes for ProtocolVersion {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let value_bytes = self.value().to_bytes()?;
        Ok(value_bytes.to_vec())
    }
}

impl FromBytes for ProtocolVersion {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (version, rem): (SemVer, &[u8]) = FromBytes::from_bytes(bytes)?;
        let protocol_version = ProtocolVersion::new(version);
        Ok((protocol_version, rem))
    }
}

#[doc(hidden)]
/// Returns `true` if a we can serialize and then deserialize a value
pub fn test_serialization_roundtrip<T>(t: &T)
where
    T: ToBytes + FromBytes + PartialEq,
{
    let serialized = ToBytes::to_bytes(t).expect("Unable to serialize data");
    let deserialized = deserialize::<T>(&serialized).expect("Unable to deserialize data");
    assert!(*t == deserialized)
}

#[allow(clippy::unnecessary_operation)]
#[cfg(test)]
mod proptests {
    use proptest::{collection::vec, prelude::*};

    use crate::{bytesrepr, gens::*};

    proptest! {
        #[test]
        fn test_u8(u in any::<u8>()) {
            bytesrepr::test_serialization_roundtrip(&u)
        }

        #[test]
        fn test_u32(u in any::<u32>()) {
            bytesrepr::test_serialization_roundtrip(&u)
        }

        #[test]
        fn test_i32(u in any::<i32>()) {
            bytesrepr::test_serialization_roundtrip(&u)
        }

        #[test]
        fn test_u64(u in any::<u64>()) {
            bytesrepr::test_serialization_roundtrip(&u)
        }

        #[test]
        fn test_u8_slice_32(s in u8_slice_32()) {
            bytesrepr::test_serialization_roundtrip(&s)
        }

        #[test]
        fn test_vec_u8(u in vec(any::<u8>(), 1..100)) {
            bytesrepr::test_serialization_roundtrip(&u)
        }

        #[test]
        fn test_vec_i32(u in vec(any::<i32>(), 1..100)) {
            bytesrepr::test_serialization_roundtrip(&u)
        }

        #[test]
        fn test_vec_vec_u8(u in vec(vec(any::<u8>(), 1..100), 10)) {
            bytesrepr::test_serialization_roundtrip(&u)
        }

        #[test]
        fn test_uref_map(m in named_keys_arb(20)) {
            bytesrepr::test_serialization_roundtrip(&m)
        }

        #[test]
        fn test_array_u8_32(arr in any::<[u8; 32]>()) {
            bytesrepr::test_serialization_roundtrip(&arr)
        }

        #[test]
        fn test_string(s in "\\PC*") {
            bytesrepr::test_serialization_roundtrip(&s)
        }

        #[test]
        fn test_option(o in proptest::option::of(key_arb())) {
            bytesrepr::test_serialization_roundtrip(&o)
        }

        #[test]
        fn test_unit(unit in Just(())) {
            bytesrepr::test_serialization_roundtrip(&unit)
        }

        #[test]
        fn test_value_account(acct in account_arb()) {
            bytesrepr::test_serialization_roundtrip(&acct)
        }

        #[test]
        fn test_u128_serialization(u in u128_arb()) {
            bytesrepr::test_serialization_roundtrip(&u);
        }

        #[test]
        fn test_u256_serialization(u in u256_arb()) {
            bytesrepr::test_serialization_roundtrip(&u);
        }

        #[test]
        fn test_u512_serialization(u in u512_arb()) {
            bytesrepr::test_serialization_roundtrip(&u);
        }

        #[test]
        fn test_key_serialization(key in key_arb()) {
            bytesrepr::test_serialization_roundtrip(&key);
        }

        #[test]
        fn test_value_serialization(v in value_arb()) {
            bytesrepr::test_serialization_roundtrip(&v);
        }

        #[test]
        fn test_access_rights(access_right in access_rights_arb()) {
            bytesrepr::test_serialization_roundtrip(&access_right)
        }

        #[test]
        fn test_uref(uref in uref_arb()) {
            bytesrepr::test_serialization_roundtrip(&uref);
        }

        #[test]
        fn test_public_key(pk in public_key_arb()) {
            bytesrepr::test_serialization_roundtrip(&pk)
        }

        #[test]
        fn test_result(result in result_arb()) {
            bytesrepr::test_serialization_roundtrip(&result)
        }

        #[test]
        fn test_phase_serialization(phase in phase_arb()) {
            bytesrepr::test_serialization_roundtrip(&phase)
        }

        #[test]
        fn test_protocol_version(protocol_version in protocol_version_arb()) {
            bytesrepr::test_serialization_roundtrip(&protocol_version)
        }

        #[test]
        fn test_sem_ver(sem_ver in sem_ver_arb()) {
            bytesrepr::test_serialization_roundtrip(&sem_ver)
        }
    }
}
