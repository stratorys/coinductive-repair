//! API utilisée pour démontrer la validation Rust → JSON → CUE.

pub mod account;

/// Additionne deux entiers dont la somme tient dans un `u32`.
///
/// Panique si la somme dépasse `u32::MAX`.
pub fn add(
    a: u32,
    b: u32,
) -> u32 {
    a.checked_add(b).expect("sum exceeds u32::MAX")
}
