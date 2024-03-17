//! Synchronization and interior mutability primitives

mod mutex;
mod up;

pub use up::UPSafeCell;
