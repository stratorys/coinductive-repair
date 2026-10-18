//! Small API used to demonstrate Rust → JSON → CUE validation.

pub mod ledger;

/// Adds two integers whose sum fits in a `u32`.
///
/// Panics if the sum exceeds `u32::MAX`.
pub fn add(
    a: u32,
    b: u32,
) -> u32 {
    a.checked_add(b).expect("sum exceeds u32::MAX")
}
