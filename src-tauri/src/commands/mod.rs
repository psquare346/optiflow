// OptiFlow — Commands module
// Re-exports all command handlers for use in lib.rs

pub mod crud;
pub mod solver_commands;
pub mod demo;

// Re-export all command functions
pub use crud::*;
pub use solver_commands::*;
pub use demo::*;
