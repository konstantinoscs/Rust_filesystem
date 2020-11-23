//! File system with block support
//!
//! Create a filesystem that only has a notion of blocks, by implementing the [`FileSysSupport`] and the [`BlockSupport`] traits together (you have no other choice, as the first one is a supertrait of the second).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
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
//! COMPLETED: YES
//!
//! COMMENTS:
//!
//! ...
//!

// We import std::error and std::format so we can say error::Error instead of
// std::error::Error, etc.
use std::path::Path;

// If you want to import things from the API crate, do so as follows:
use bit_field::BitField;
use cplfs_api::controller::Device;
use cplfs_api::fs::BlockSupport;
use cplfs_api::fs::FileSysSupport;
use cplfs_api::types::{Block, SuperBlock, DINODE_SIZE};

use super::error_fs::BlockLayerError;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out your file system name.
pub type FSName = BlockLayerFS;

/// Struct representing the block layer
#[derive(Debug)]
pub struct BlockLayerFS {
    ///the SuperBlock for fast access
    super_block: SuperBlock,

    /// the encapsulated device
    device: Device,
}

/// Functions specific to BlockLayerFS
impl BlockLayerFS {
    /// Returns a reference to the Filesystem's cached superblock
    pub fn sup_as_ref(&self) -> &SuperBlock {
        &self.super_block
    }
}

impl FileSysSupport for BlockLayerFS {
    type Error = BlockLayerError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        let inode_blocks =
            (sb.ninodes as f64 / (sb.block_size / *DINODE_SIZE) as f64).ceil() as u64;
        // ((*DINODE_SIZE * sb.ninodes) as f64 / sb.block_size as f64).ceil() as u64;
        let bmap_blocks = (sb.ndatablocks as f64 / (sb.block_size * 8) as f64).ceil() as u64;
        sb.inodestart == 1
            && sb.inodestart + inode_blocks - 1 < sb.bmapstart
            && sb.bmapstart + bmap_blocks - 1 < sb.datastart
            && sb.datastart + sb.ndatablocks - 1 < sb.nblocks
    }

    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error> {
        match Self::sb_valid(sb) {
            false => Err(BlockLayerError::BlockLayerInput("SuperBlock not valid")),
            true => {
                let mut device = Device::new(path, sb.block_size, sb.nblocks)?;
                let mut super_block = Block::new_zero(0, sb.block_size);
                super_block.serialize_into(sb, 0)?;
                device.write_block(&super_block)?;
                Ok(BlockLayerFS {
                    super_block: SuperBlock::from(*sb),
                    device,
                })
            }
        }
    }

    fn mountfs(dev: Device) -> Result<Self, Self::Error> {
        let sblock = dev.read_block(0)?;
        let super_block = sblock.deserialize_from::<SuperBlock>(0)?;
        match Self::sb_valid(&super_block) {
            false => Err(BlockLayerError::BlockLayerInput("SuperBlock not valid")),
            true => Ok(BlockLayerFS {
                super_block,
                device: dev,
            }),
        }
    }

    fn unmountfs(self) -> Device {
        self.device
    }
}

impl BlockSupport for BlockLayerFS {
    fn b_get(&self, i: u64) -> Result<Block, Self::Error> {
        Ok(self.device.read_block(i)?)
    }

    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error> {
        Ok(self.device.write_block(b)?)
    }

    fn b_free(&mut self, i: u64) -> Result<(), Self::Error> {
        let byte_size = 8;
        let t_block_addr =
            self.super_block.bmapstart + i / (self.super_block.block_size * byte_size);
        if t_block_addr >= self.super_block.datastart {
            return Err(BlockLayerError::BlockLayerInput(
                "Block address is outside bitmap bounds",
            ));
        }
        //how many bits inside the target block we have to look
        let block_offset_bit = i % (self.super_block.block_size * byte_size);
        //offset of the byte inside the target_block
        let target_byte = block_offset_bit / byte_size;
        let target_bit = block_offset_bit % byte_size;
        //get bitmap starting address, divide i/blocksize and then
        let mut target_block = self.b_get(t_block_addr)?;
        //byte that will contain the bit we want to change
        let mut byte_slice: [u8; 1] = Default::default();
        target_block.read_data(&mut byte_slice, target_byte)?;
        let byte = byte_slice.first_mut().unwrap();
        match byte.get_bit(target_bit as usize) {
            false => {
                return Err(BlockLayerError::BlockLayerWrite(
                    "Trying to free a free block",
                ))
            }
            true => byte.set_bit(target_bit as usize, false),
        };

        //write back
        target_block.write_data(&byte_slice, target_byte)?;
        self.b_put(&target_block)?;
        Ok(())
    }

    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error> {
        if i > self.super_block.ndatablocks - 1 {
            return Err(BlockLayerError::BlockLayerInput(
                "Trying to access a block with index outside bounds",
            ));
        }
        let block_len = self.b_get(self.super_block.datastart + i)?.len();
        let zero_block = Block::new_zero(self.super_block.datastart + i, block_len);
        self.b_put(&zero_block)
    }

    fn b_alloc(&mut self) -> Result<u64, Self::Error> {
        let bmap_blocks = (self.super_block.ndatablocks as f64 / 8.0).ceil() as u64;
        let mut bit: u64 = 0;
        let mut byte_slice: [u8; 1] = Default::default();
        // iterate over every block to find a free bit
        for bl in 0..bmap_blocks {
            let mut block = self.b_get(self.super_block.bmapstart + bl)?;
            let buf = block.contents_as_ref();
            //iterate over every byte and count the bits until we find a "0"
            for by in 0..block.len() {
                if buf[by as usize] != 0b1111_1111 {
                    // iterate inside the byte
                    for i in 0..8 {
                        //the byte may have padding and go to illegal addresses so we check
                        if bit + i == self.super_block.ndatablocks {
                            return Err(BlockLayerError::BlockLayerOp("No space left!"));
                        }
                        //if zero bit is found, write the block and persist it
                        if !buf[by as usize].get_bit(i as usize) {
                            block.read_data(&mut byte_slice, by)?;
                            byte_slice.first_mut().unwrap().set_bit(i as usize, true);
                            block.write_data(&byte_slice, by)?;
                            self.b_put(&block)?;
                            return Ok(bit + i as u64);
                        }
                    }
                } else {
                    //no free spot was found, iterate one byte
                    bit += 8;
                }
            }
        }
        Err(BlockLayerError::BlockLayerOp("No space left!"))
    }

    fn sup_get(&self) -> Result<SuperBlock, Self::Error> {
        Ok(SuperBlock::from(self.super_block))
    }

    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error> {
        let mut super_block = self.device.read_block(0)?;
        super_block.serialize_into(sup, 0)?;
        self.device.write_block(&super_block)?;
        self.super_block = SuperBlock::from(*sup);
        Ok(())
    }
}

// Here we define a submodule, called `tests`, that will contain our unit tests
// Take a look at the specified path to figure out which tests your code has to pass.
// As with all other files in the assignment, the testing module for this file is stored in the API crate (this is the reason for the 'path' attribute in the code below)
// The reason I set it up like this is that it allows me to easily add additional tests when grading your projects, without changing any of your files, but you can still run my tests together with yours by specifying the right features (see below) :)
// directory.
//
// To run these tests, run the command `cargo test --features="X"` in the `solution` directory, with "X" a space-separated string of the features you are interested in testing.
//
// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
//The below configuration tag specifies the following things:
// 'cfg' ensures this module is only included in the source if all conditions are met
// 'all' is true iff ALL conditions in the tuple hold
// 'test' is only true when running 'cargo test', not 'cargo build'
// 'any' is true iff SOME condition in the tuple holds
// 'feature = X' ensures that the code is only compiled when the cargo command includes the flag '--features "<some-features>"' and some features includes X.
// I declared the necessary features in Cargo.toml
// (Hint: this hacking using features is not idiomatic behavior, but it allows you to run your own tests without getting errors on mine, for parts that have not been implemented yet)
// The reason for this setup is that you can opt-in to tests, rather than getting errors at compilation time if you have not implemented something.
// The "a" feature will run these tests specifically, and the "all" feature will run all tests.
#[cfg(all(test, any(feature = "a", feature = "all")))]
#[path = "../../api/fs-tests/a_test.rs"]
mod tests;
