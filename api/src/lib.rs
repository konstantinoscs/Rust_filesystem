//! The API you must implement
//!
//! This crate contains the definitions of the various traits you will
//! implement and some basic types.
//! The contents of this crate are discussed in more detail in the assignment
//! **You are not allowed to edit this crate in any way.**
//!
//! Placing the modules here ensures that Cargo notices them as part of the build process.

#![deny(missing_docs)]

//Implementation of the controller layer
pub mod controller;
pub mod error_given;

//Basic modules for types
pub mod types;

//Traits you should implement
pub mod fs;
