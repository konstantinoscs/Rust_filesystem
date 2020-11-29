//! File system with path support
//!
//! Create a filesystem that has a notion of blocks, inodes, directory inodes and paths, by implementing the [`FileSysSupport`], the [`BlockSupport`], the [`InodeSupport`], the [`DirectorySupport`] and the [`PathSupport`] traits together (again, all earlier traits are supertraits of the later ones).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
//! [`DirectorySupport`]: ../../cplfs_api/fs/trait.DirectorySupport.html
//! [`PathSupport`]: ../../cplfs_api/fs/trait.PathSupport.html
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

use cplfs_api::types::{FType, SuperBlock, Block, Inode};
use cplfs_api::fs::{FileSysSupport, BlockSupport, InodeSupport};
use std::path::Path;
use cplfs_api::controller::Device;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
/// **TODO**: replace the below type by the type of your file system
pub type FSName = ();

/*///Struct representing a file system with up to Directory layer support
#[derive(Debug)]
pub enum PathFS {}

impl FileSysSupport for PathFS {
    type Error = DirLayerError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        unimplemented!()
    }

    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error> {
        unimplemented!()
    }

    fn mountfs(dev: Device) -> Result<Self, Self::Error> {
        unimplemented!()
    }

    fn unmountfs(self) -> Device {
        unimplemented!()
    }
}

impl BlockSupport for PathFS {
    fn b_get(&self, i: u64) -> Result<Block, Self::Error> {
        unimplemented!()
    }

    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn b_free(&mut self, i: u64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn b_alloc(&mut self) -> Result<u64, Self::Error> {
        unimplemented!()
    }

    fn sup_get(&self) -> Result<SuperBlock, Self::Error> {
        unimplemented!()
    }

    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

impl InodeSupport for PathFS {
    type Inode = Inode;

    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error> {
        unimplemented!()
    }

    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn i_free(&mut self, i: u64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error> {
        unimplemented!()
    }

    fn i_trunc(&mut self, inode: &mut Self::Inode) -> Result<(), Self::Error> {
        unimplemented!()
    }
}*/

// **TODO** define your own tests here.

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "d", feature = "all")))]
#[path = "../../api/fs-tests/d_test.rs"]
mod tests;
