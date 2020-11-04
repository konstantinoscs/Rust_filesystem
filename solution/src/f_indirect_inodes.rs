//! File system with support for inodes that have indirect blocks too.
//! Reimplementation of the inodes from the base project.
//! This additional assignment requires completion of assignment e, so go and do that one first if you haven't yet.
//!
//! Create a filesystem that has a notion of inodes and blocks, by implementing the [`FileSysSupport`], the [`BlockSupport`], the [`InodeSupport`] and the [`InodeRWSupport`] traits together (again, all earlier traits are supertraits of the later ones).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
//! [`InodeRWSupport`]: ../../cplfs_api/fs/trait.InodeRWSupport.html
//!
//! However, this time, you **cannot** use the `DInode` type I provided, but rather, you have to define your own type,  have it derive all the traits you need (check the `#[derive(...)]` for `DInode` in the provided API code), wrap it in your own `Inode` type, and have your own `Inode` type implement the `InodeLike` trait, so that it is still compatible with the tests.
//! So far, we have only supported inodes that have at most a fixed, direct number of data blocks (i.e. `DIRECT_POINTERS` data blocks) associated to them.
//! Reimplement your solution for the base project, but now support inodes that have an extra field (let's call this field the *indirect block* field), that points to a single data block; the so-called *indirect block*.
//! As long as your inode requires `DIRECT_POINTERS` data blocks or fewer, all code behaves as before and the indirect block field is set to 0.
//! As soon as block number `DIRECT_POINTERS+1` has to be allocated, the indirect block gets allocated along with it, to store a sequence of block numbers.
//! The address of block `DIRECT_POINTERS+1` gets stored as the first address in the indirect block.
//! Later, when data block `DIRECT_POINTERS+2` has to be allocated, it gets stored in the second slot on the indirect block, and so on. In other words, given that inode numbers are of type `u64` and should in principle be represented by 8 bytes at runtime, this indirect block allows files to allocate another `block_size/8` blocks.
//!
//! Some more specific pointers for the implementation:
//! - Try to come up with some helper functions that make it more convenient to work with indirect blocks
//! - Think of the scenario's where an indirect block might get (de)allocated. See if you can wrap this allocation in one of these previously mentioned helper functions.
//! - Since the `new` method in `InodeLike` is static and does not allow you to allocate new blocks, it should still not allow you to provide more than `DIRECT_POINTERS+1` blocks. The last block is then the block number of the indirect block. Similarly, the `get_block` method should simply return the number of the indirect block when queried for block *index* `DIRECT_POINTERS`, since there is no way for inodes to read from the device to figure out the actual block number.
//! - Do not forget to deallocate the indirect block itself, when truncating or freeing an inode.
//!
//! It should be possible to swap out the inodes you use in your filesystem so far without making any of the tests for previous assignments fail.
//! You could do this (rather than copying all of your code and starting over) if you want some extra assurance that your implementation is still correct (or at least, still correct when not indexing inodes past the `DIRECT_POINTERS`th block)
//! At the end, write some tests that convincingly show that your implementation indeed supports indirect pointers.
//!
//! Make sure this file does not contain any unaddressed `TODO`s anymore when you hand it in.
//!
//! # Status
//!
//! **TODO**: Replace the question mark below with YES, NO, or PARTIAL to
//! indicate the status of this assignment. If you want to tell something
//! about this assignment to the grader, e.g., you have a bug you can't fix,
//! or you want to explain your approach, write it down after the comments
//! section. If you had no major issues and everything works, there is no need to write any comments.
//!
//! COMPLETED: ?
//!
//! COMMENTS:
//!
//! ...
//!

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
/// **TODO**: replace the below type by the type of your file system
pub type FSName = ();

// **TODO** define your own tests here.

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "f", feature = "all")))]
#[path = "../../api/fs-tests/f_test.rs"]
mod tests;
