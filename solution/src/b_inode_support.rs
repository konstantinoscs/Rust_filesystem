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

    ///returns the block that contains inode with index i
    fn get_block_of_inode(&self, i: u64) -> Result<Block, InodeLayerError> {
        if i > self.sup_as_ref().ninodes - 1 {
            return Err(InodeLayerError::InodeLayerInput(
                "Trying to get inode with index out of bounds",
            ));
        }
        let t_block_addr = self.sup_as_ref().inodestart + i / self.inodes_per_block;
        self.b_get(t_block_addr)
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
        let t_offset = (i % self.inodes_per_block) * (*DINODE_SIZE);
        let target_block = self.get_block_of_inode(i)?;
        let di_node = target_block.deserialize_from::<DInode>(t_offset)?;
        Ok(Inode {
            inum: i,
            disk_node: di_node,
        })
    }

    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {
        let t_offset = (ino.inum % self.inodes_per_block) * (*DINODE_SIZE);
        let mut target_block = self.get_block_of_inode(ino.inum)?;
        target_block.serialize_into(&ino.disk_node, t_offset)?;
        self.b_put(&target_block)?;
        Ok(())
    }

    fn i_free(&mut self, i: u64) -> Result<(), Self::Error> {
        let mut inode = self.i_get(i)?;
        if inode.disk_node.ft == FType::TFree {
            return Err(InodeLayerError::InodeLayerOp(
                "Trying to free a TFree inode",
            ));
        }
        if inode.disk_node.nlink != 0 {
            return Ok(());
        }
        inode.disk_node.ft = FType::TFree;
        let blocks_occupied =
            (inode.disk_node.size as f64 / self.sup_as_ref().block_size as f64).ceil() as u64;
        for i in 0..blocks_occupied {
            //calculate the relative address to datastart as required by b_free
            let target_block =
                inode.disk_node.direct_blocks[i as usize] - self.sup_as_ref().datastart;
            self.block_fs.b_free(target_block)?;
            inode.disk_node.direct_blocks[i as usize] = 0;
        }
        self.i_put(&inode)
    }

    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error> {
        let inode_blocks =
            (self.sup_as_ref().ninodes as f64 / self.inodes_per_block as f64).ceil() as u64;
        let mut nodes_searched = 1;
        for bl in 0..inode_blocks {
            let mut block = self.block_fs.b_get(self.sup_as_ref().inodestart + bl)?;
            for node in 0..self.inodes_per_block {
                if bl == 0 && node == 0 {
                    //skip root inode
                    continue;
                }
                if nodes_searched == self.sup_as_ref().ninodes {
                    break;
                }
                let mut di_node = block.deserialize_from::<DInode>(node * (*DINODE_SIZE))?;
                if di_node.ft == FType::TFree {
                    di_node.ft = ft;
                    di_node.size = 0;
                    di_node.nlink = 0;
                    block.serialize_into(&di_node, node * (*DINODE_SIZE))?;
                    self.block_fs.b_put(&block)?;
                    return Ok(nodes_searched);
                }
                nodes_searched += 1;
            }
        }
        Err(InodeLayerError::InodeLayerOp(
            "Cannot allocate new block, no space left!",
        ))
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
