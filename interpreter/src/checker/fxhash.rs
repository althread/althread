use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
};

/// Fast non-cryptographic hasher (FxHash) for internal model-checker tables.
#[derive(Default)]
pub(crate) struct FxHasher {
    hash: usize,
}

#[cfg(target_pointer_width = "64")]
const K: usize = 0x517cc1b727220a95;
#[cfg(target_pointer_width = "32")]
const K: usize = 0x9e3779b9;

impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, value: usize) {
        self.hash = self.hash.rotate_left(5).bitxor(value).wrapping_mul(K);
    }
}

use std::ops::BitXor;

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        #[cfg(target_pointer_width = "64")]
        while bytes.len() >= 8 {
            let chunk = usize::from_ne_bytes(bytes[..8].try_into().unwrap());
            self.add_to_hash(chunk);
            bytes = &bytes[8..];
        }
        while bytes.len() >= 4 {
            let chunk = u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as usize;
            self.add_to_hash(chunk);
            bytes = &bytes[4..];
        }
        for &byte in bytes {
            self.add_to_hash(byte as usize);
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        #[cfg(target_pointer_width = "64")]
        self.add_to_hash(i as usize);
        #[cfg(target_pointer_width = "32")]
        {
            self.add_to_hash(i as usize);
            self.add_to_hash((i >> 32) as usize);
        }
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash as u64
    }
}

pub(crate) type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
