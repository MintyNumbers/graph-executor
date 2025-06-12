use std::{mem::size_of, slice::from_raw_parts};

/// Trait for casting sized structs as bytes and vice versa.
pub trait AsFromBytes: Sized + Clone {
    /// Cast struct as bytes.
    fn as_bytes(&self) -> &[u8] {
        unsafe { from_raw_parts((self as *const Self) as *const u8, size_of::<Self>()) }
    }

    /// Casts byte representation as the implementing struct's type.
    fn from_bytes(bytes: &[u8]) -> Self {
        // unsafe { core::mem::transmute_copy(&bytes[0]) }
        unsafe { std::ptr::read(bytes.as_ptr() as *const Self) }
        // unsafe { &*(bytes.as_ptr() as *const Self) }.to_owned() // used to work
    }
}
