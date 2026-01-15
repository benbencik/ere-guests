//! Stateless validator common types and utilities.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod guest;
pub mod new_payload_request;

#[cfg(feature = "rkyv")]
pub mod rkyv_wrappers;

#[cfg(feature = "serde")]
pub mod serde_wrappers;

#[cfg(feature = "host")]
pub mod host;
