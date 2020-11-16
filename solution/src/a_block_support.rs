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

///Functions specific to BlockLayerFS
impl BlockLayerFS {
    ///Returns a reference to the Filesystem's cached superblock
    pub fn sup_as_ref(&self) -> &SuperBlock {
        &self.super_block
    }
}

impl FileSysSupport for BlockLayerFS {
    type Error = BlockLayerError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        let inode_blocks =
            ((*DINODE_SIZE * sb.ninodes) as f64 / sb.block_size as f64).ceil() as u64;
        sb.inodestart == 1
            && sb.inodestart + inode_blocks - 1 < sb.bmapstart
            && sb.bmapstart < sb.datastart
            && sb.datastart < sb.nblocks - sb.ndatablocks + 1
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
        let block_addr = self.super_block.bmapstart + i / (self.super_block.block_size * byte_size);
        if block_addr >= self.super_block.datastart {
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
        let mut target_block = self.b_get(block_addr)?;
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
        let full_byte: u8 = 0b11111111;
        let mut byte_slice: [u8; 1] = Default::default();
        // iterate over every block to find a free bit
        for bl in 0..bmap_blocks {
            let mut block = self.b_get(self.super_block.bmapstart + bl)?;
            let buf = block.contents_as_ref();
            //iterate over every byte and count the bits until we find a "0"
            for by in 0..block.len() {
                if buf[by as usize] ^ full_byte != 0 {
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

// Here we define a submodule, called `my_tests`, that will contain your unit
// tests for this module.
// **TODO** define your own tests here. I have written down one test as an example of the syntax.
// You can define more tests in different modules, and change the name of this module
//
// The `test` in the `#[cfg(test)]` annotation ensures that this code is only compiled when we're testing the code.
// To run these tests, run the command `cargo test` in the `solution` directory
//
// To learn more about testing, check the Testing chapter of the Rust
// Book: https://doc.rust-lang.org/book/testing.html
#[cfg(test)]
mod my_tests {

    #[test]
    fn trivial_unit_test() {
        assert_eq!(2, 2);
        assert!(true);
    }
}

// If you want to write more complicated tests that create actual files on your system, take a look at `utils.rs` in the assignment, and how it is used in the `fs_tests` folder to perform the tests. I have imported it below to show you how it can be used.
// The `utils` folder has a few other useful methods too (nothing too crazy though, you might want to write your own utility functions, or use a testing framework in rust, if you want more advanced features)
#[cfg(test)]
#[path = "../../api/fs-tests"]
mod test_with_utils {

    #[path = "utils.rs"]
    mod utils;

    #[test]
    fn unit_test() {
        //The below method set up the parent folder "a_parent_unique_name" within the root directory  of this solution crate
        //Also delete the file "image_file" within this folder if it already exists, so that it does not interfere with any later `mkfs` calls (this is useful if your previous test run failed, and the file did not get deleted)
        //*WARNING* !Make sure that this folder name "a_parent_unique_name" is actually unique over different tests, because tests are executed in parallel by default!
        //Returns the concatenated path, so that you can use the path further on, e.g. when creating a `Device` or `FileSystem`

        //! `let path = utils::disk_prep_path("a_parent_unique_name", "image_file");`

        //Things you want to test go here (check my tests in the API folder for examples)
        //! ...
        //! ...

        // If some disk actually created the file under `path` in your code, then you can uncomment the following call to clean it up:
        //!  `utils::disk_unprep_path(&path);`
        // This removes the image file and the parent directory at the end, so that no garbage is left in your file system
        //*WARNING* if a Device `dev` is still in scope for the path `path`, then the above call will block (the device holds a lock on the memory-mapped file)
        //You then have to use the following call instead:

        //! `utils::disk_destruct(dev);`

        //This makes the device go out of scope first, before tearing down the parent folder and image file, thereby avoiding deadlock
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
