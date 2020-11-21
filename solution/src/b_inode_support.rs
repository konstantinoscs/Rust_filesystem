//! File system with inode support
//!
//! Create a filesystem that has a notion of inodes and blocks, by implementing the [`FileSysSupport`], the [`BlockSupport`] and the [`InodeSupport`] traits together (again, all earlier traits are supertraits of the later ones).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
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

use cplfs_api::controller::Device;
use cplfs_api::fs::{BlockSupport, FileSysSupport, InodeSupport};
use cplfs_api::types::{Block, DInode, FType, Inode, SuperBlock, DINODE_SIZE};
use std::path::Path;

use super::a_block_support::BlockLayerFS;
use super::error_fs::InodeLayerError;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
pub type FSName = InodeLayerFS;

///Struct representing a file system with up to Inode layer support
#[derive(Debug)]
pub struct InodeLayerFS {
    block_fs: BlockLayerFS,
    inodes_per_block: u64,
}

/// Functions specific to InodeLayerFS
impl InodeLayerFS {
    /// Returns a reference to the Filesystem's cached superblock
    pub fn sup_as_ref(&self) -> &SuperBlock {
        self.block_fs.sup_as_ref()
    }
}

impl FileSysSupport for InodeLayerFS {
    type Error = InodeLayerError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        BlockLayerFS::sb_valid(sb)
    }

    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error> {
        let mut block_fs = BlockLayerFS::mkfs(path, sb)?;

        let inodes_per_block = sb.block_size / *DINODE_SIZE;
        let inode_blocks = (sb.ninodes as f64 / inodes_per_block as f64).ceil() as u64;
        let mut nodes_init = 0;
        let default_dinode = DInode::default();
        assert_eq!(default_dinode.ft, FType::TFree);
        //init every inode as TFree
        for bl in 0..inode_blocks {
            let mut block = block_fs.b_get(sb.inodestart + bl)?;
            for node in 0..inodes_per_block {
                if nodes_init == sb.ninodes {
                    break;
                }
                //println!("Writing inode with offset {}", node*(*DINODE_SIZE));
                block.serialize_into(&default_dinode, node * (*DINODE_SIZE))?;
                nodes_init += 1;
            }
            block_fs.b_put(&block)?;
        }

        Ok(InodeLayerFS {
            block_fs,
            inodes_per_block,
        })
    }

    fn mountfs(dev: Device) -> Result<Self, Self::Error> {
        let block_fs = BlockLayerFS::mountfs(dev)?;
        let inodes_per_block = block_fs.sup_as_ref().block_size / *DINODE_SIZE;
        Ok(InodeLayerFS {
            block_fs,
            inodes_per_block,
        })
    }

    fn unmountfs(self) -> Device {
        self.block_fs.unmountfs()
    }
}

impl BlockSupport for InodeLayerFS {
    fn b_get(&self, i: u64) -> Result<Block, Self::Error> {
        Ok(self.block_fs.b_get(i)?)
    }

    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error> {
        Ok(self.block_fs.b_put(b)?)
    }

    fn b_free(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.block_fs.b_free(i)?)
    }

    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.block_fs.b_zero(i)?)
    }

    fn b_alloc(&mut self) -> Result<u64, Self::Error> {
        Ok(self.block_fs.b_alloc()?)
    }

    fn sup_get(&self) -> Result<SuperBlock, Self::Error> {
        Ok(self.block_fs.sup_get()?)
    }

    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error> {
        Ok(self.block_fs.sup_put(sup)?)
    }
}

impl InodeSupport for InodeLayerFS {
    type Inode = Inode;

    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error> {
        if i > self.sup_as_ref().ninodes - 1 {
            return Err(InodeLayerError::InodeLayerInput(
                "Trying to get inode with index out of bounds",
            ));
        }
        let t_block_addr = self.sup_as_ref().inodestart + i / self.inodes_per_block;
        let t_offset = (i % self.inodes_per_block) * (*DINODE_SIZE);
        println!(
            "Getting i {}, translating it to t_block_addr {} and offset {}",
            i, t_block_addr, t_offset
        );
        let mut target_block = self.block_fs.b_get(t_block_addr)?;
        let di_node = target_block.deserialize_from::<DInode>(t_offset)?;
        target_block.serialize_into(&di_node, t_offset)?;
        Ok(Inode {
            inum: i,
            disk_node: di_node,
        })
    }

    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {
        let t_block_addr = self.sup_as_ref().inodestart + ino.inum / self.inodes_per_block;
        let t_offset = (ino.inum % self.inodes_per_block) * (*DINODE_SIZE);
        println!(
            "Putting i {}, translating it to t_block_addr {} and offset {}",
            ino.inum, t_block_addr, t_offset
        );
        let mut target_block = self.b_get(t_block_addr)?;
        target_block.serialize_into(&ino.disk_node, t_offset)?;
        self.b_put(&target_block)?;
        Ok(())
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
}

// **TODO** define your own tests here.

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "b", feature = "all")))]
#[path = "../../api/fs-tests/b_test.rs"]
mod tests;
