//! Root file of your solution
//!
//! # This file
//!
//! There are two types of projects or *crates* in Rust: libraries and
//! executables. The difference should be obvious. The root of a library
//! project is `src/lib.rs` (like this file) and the root of an executable
//! project is `src/main.rs`.
//!
//! The root file contains some important things:
//!
//! 1. It provides some documentation for the crate. You are reading it right
//!    now.
//! 2. It contains annotations that are true for the whole crate. See the
//!    source code of this file for an example.
//! 3. It declares dependencies on external crates. Again, see the source
//!    code.
//! 4. It declares the modules of which the crate consists using the `mod`
//!    keyword. Don't confuse this with the `use` keyword. If you add any
//!    modules to this project, you have to declare at the end of this file
//!    using the `mod` keyword. Unless they are submodules of another module,
//!    in which case you have to declarate at the top of that module. In case
//!    you want them to be public (and visible in the documentation too), use
//!    `pub mod` instead of `mod`.
//!
//! See the chapter on [Crates and Modules] in the Rust
//! book for more information.
//!
//! [Crates and Modules]: https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
//!
//! The source code of this file contains more information.
//!
//! If you are looking at the documentation of this file, the list on the left
//! side contains all the crates that this crate transitively depends on.
//!
//! The next thing to look at is the [`a_block_support` module](a_block_support/index.html).
//!
//! # Rust Version
//!
//! **TODO**: indicate which *stable* version of Rust you are using. If you are using
//! 1.47, you don't have to do anything. Otherwise, replace the version
//! below with the output of `rustc --version`.
//!
//! VERSION: rustc 1.47 (2020-10-08)

// This line forces you to write documentation for all important things.
#![deny(missing_docs)]
// Note that the documentation starts with three slashes instead of two!
// See https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#making-useful-documentation-comments

// Declare the modules of which this project consists:

// Mandatory assignments
pub mod a_block_support;
pub mod b_inode_support;
pub mod c_dirs_support;

// Optional assignments
pub mod d_path_support;
#[allow(non_snake_case)]
pub mod e_inode_RW_support;
pub mod f_indirect_inodes;
pub mod g_caching_inodes;

// Declare additional modules below or declare them in other modules.
