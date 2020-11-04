//! Implementation of a physical disk and a very simple device controller for it.
//! The device and its contents are represented by a file in your file system, that is memory mapped and stored in a Device struct.
//! When initializing the controller, you have to provide it with either a path to a non-existing file, which will then be created and used as the contents of your device, or to an existing file, which will be opened and the contents of which will be checked.
//! Provides a basic block read and write operation on a device at a given offset.
//! The memory-mapped file is what the read and write functions operate on.
//!
//! *EXTRA*: Note that this explicit block-level abstraction is not required for a file system at this level of abstraction, but added it to make our model a more realistic representation of a real-life file system.
//! No provisions have been made to properly lock and unlock the file that is used to back the file system, so do not fiddle with it while a file system is running, as this leads to undefined behavior. (e.g. the fs2 crate could be used to explicitly implement locking, if so desired)

use super::error_given;
use super::error_given::APIError;
use super::types::Block;
use memmap::MmapMut;
use std::{
    fs::{remove_file, OpenOptions},
    path::{Path, PathBuf},
};

/// Struct representing the state of a hard drive disk (HDD).
/// The implementation of this structure is the controller that allows us to read disk blocks from the disk, and write disk blocks to the disk.
///
/// *EXTRA*: As a side note, it would be nicer if we could make both the `Device` and the `Block` polymorphic in `block_size` i.e. add `<block_size : u64>` and write the signature of e.g. the `read_block` function as:
/// `read_block(&self, index: u64) -> anyhow::Result<Block<block_size>>` with self now of type `Device<block_size>`
/// This would statically enforce blocks to have the right size when they are read or written.
/// This type of genericism, which can be seen as a lightweight version of dependent types, is also referred to as *const generics*.
/// No const generics possible in stable rust yet, to parameterize over block_size.
/// This sort of functionality can be achieved using templates in C++, and is also available in Rust's nightly build, but as of yet not in the stable fraction of Rust.
#[derive(Debug)]
pub struct Device {
    /// Size of the blocks that this disk reads and writes
    pub block_size: u64,
    /// Total number of blocks this disk consists of
    pub nblocks: u64,
    /// Path to the file in your file system that is used as a storage area to emulate the disk
    path: PathBuf,
    /// Memory-mapped contents of the above file. This is what is manipulated in the read and write functions.
    contents: MmapMut,
}

/// Small enum, used to specify whether we expect to open a new file system
#[derive(PartialEq, Eq, Copy, Clone)]
pub enum DiskState {
    /// Creating a new disk image
    New,
    /// Loading an old disk image
    Load,
}

// Import the components of this enum, so we can reuse them here
use self::DiskState::*;
impl DiskState {
    /// Convert a boolean to a `DiskState`
    pub fn new(ex: bool) -> DiskState {
        match ex {
            true => Load,
            false => New,
        }
    }
}

impl Drop for Device {
    /// This implementation of drop makes sure all writes are persisted at the end, before we release ownership of our device and its controller
    /// We only need to persist these writes if the file backing this disk actually still exists
    fn drop(&mut self) {
        if self.path.exists() {
            self.contents.flush().unwrap();
        }
    }
}

impl Device {
    /// Core function to that handles both `new` and `load`, based on the value of the switch `ds`, representing whether we want to load or create a disk
    pub fn create_device<P: AsRef<Path>>(
        path: P,
        block_size: u64,
        nblocks: u64,
        ds: DiskState,
    ) -> error_given::Result<Device> {
        let path_buf = path.as_ref().to_path_buf();
        let mmapf = mmap_path(path, block_size * nblocks, ds)?;
        Ok(Device {
            block_size: block_size,
            nblocks: nblocks,
            path: path_buf,
            contents: mmapf,
        })
    }

    /// Create a *new* disk device, given:
    /// - A `path` to store its image
    /// - A `block_size` to define the size of each unit to be read or written, in bytes
    /// - The total number of blocks in the disk
    /// This new device will have contents 0 at each address.
    ///
    /// Note that if `block_size` is smaller than the size of the main types (for the super block, inodes, etc.) used in this assignment, the file system will crash at runtime.
    /// We did not program our code defensively to cope with this.
    /// This function will return an error, if the file represented by `path` already exists.
    pub fn new<P: AsRef<Path>>(
        path: P,
        block_size: u64,
        nblocks: u64,
    ) -> error_given::Result<Device> {
        Device::create_device(path, block_size, nblocks, New)
    }

    /// Load an *existing* disk device, given its `block_size` and the number of blocks its file system ought to contain.
    /// This function will return an error, if the file represented by `path` does not yet exist.
    pub fn load<P: AsRef<Path>>(
        path: P,
        block_size: u64,
        nblocks: u64,
    ) -> error_given::Result<Device> {
        Device::create_device(path, block_size, nblocks, Load)
    }

    /// End the lifetime of this disk, and remove the file backing it on disk
    /// Assumes that you have not made any other links to the backing file
    /// Panics if removing the file fails
    pub fn destruct(self) {
        remove_file(&self.path).unwrap();
    }

    /// Size of this device in bytes
    pub fn device_size(&self) -> u64 {
        self.block_size * self.nblocks
    }

    /// Path of the file backing this device
    pub fn device_path(&self) -> &Path {
        &self.path
    }

    fn index_to_addr(&self, index: u64) -> u64 {
        self.block_size * index
    }

    /// Read `nb` bytes from the device starting at address `addr`
    /// Results in an error if a write past the end of the device is attempted
    /// Note that this function would probably not be offered in this way by a realistic device driver.
    /// Rather, the reads happen on a block-by-block basis (possibly batched)
    fn read(&self, addr: u64, nb: u64) -> error_given::Result<Box<[u8]>> {
        if addr + nb > self.device_size() {
            return Err(APIError::ControllerInput("Read past the end of the device"));
        }
        let start = addr as usize;
        let end = (addr + nb) as usize;
        Ok(self.contents[start..end].into()) //Note: this can theoretically still cause runtime errors
    }

    /// Read the block with index `index` from the device
    /// Results in an error if the block index is too high
    /// The block is returned in the form of a `Block` structure
    pub fn read_block(&self, index: u64) -> error_given::Result<Block> {
        let addr = self.index_to_addr(index);
        let block_data = self.read(addr, self.block_size)?;
        Ok(Block::new(index, block_data))
    }

    /// Write the given buffer into memory, if it does not cause a device overflow
    /// Fails if a write past the end of the device is attempted
    /// Note that this function would probably not be offered in this way by a realistic device driver.
    /// Rather, the writes happen on a block-by-block basis (possibly batched)
    fn write(&mut self, addr: u64, b: &[u8]) -> error_given::Result<()> {
        if addr + b.len() as u64 > self.device_size() {
            return Err(APIError::ControllerInput(
                "Write past the end of the device",
            ));
        }
        let start = addr as usize;
        let end = (addr as usize) + b.len();
        self.contents[start..end].copy_from_slice(b);
        Ok(())
    }

    /// Write a given block `buf` into the device at index `index`
    /// Fails if `buf` is not exactly block-sized, or if the provided index is too high
    pub fn write_block(&mut self, b: &Block) -> error_given::Result<()> {
        if b.len() != self.block_size {
            return Err(APIError::ControllerInput(
                "Trying to write a non-block-sized block",
            ));
        }
        let addr = self.index_to_addr(b.block_no);
        self.write(addr, &b.contents_as_ref())
    }
}

/// Either open or create the specified file path.
/// The boolean `ex` specifies
/// If the path already exists, check that the device represented by it has the correct size
/// If any one of the intermediate calls fails, the result of this method is not an actual device file
fn mmap_path<P: AsRef<Path>>(path: P, dsize: u64, ex: DiskState) -> error_given::Result<MmapMut> {
    let exists = DiskState::new(path.as_ref().exists());
    if exists != ex {
        if ex == Load {
            return Err(APIError::ControllerInput(
                "Tried to load a non-existing file path",
            ));
        } else {
            return Err(APIError::ControllerInput(
                "Tried to create a pre-existing file path",
            ));
        }
    }

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;

    if ex == Load {
        if f.metadata()?.len() != dsize {
            return Err(APIError::ControllerInput(
                "Device size does not match provided size",
            ));
        }
    } else {
        f.set_len(dsize)?; // The file will be extended to dsize and have all of the intermediate data filled in with 0s.
    }

    let data = unsafe { memmap::MmapOptions::new().map_mut(&f)? };
    Ok(data)
}

// Here we define a submodule, called `tests`, that will contain the unit
// tests of this module.
//
// The `#[cfg(test)]` annotation means that this code is only compiled when
// we're testing the code.
//
// To run these tests, run the command `cargo test` in the `solution`
// directory.
//
// To learn more about testing, check the Testing chapter of the Rust
// Book: https://doc.rust-lang.org/book/ch11-00-testing.html
//
// **VERY IMPORTANT NOTE** if you want to implement scenerio tests yourself; Rust runs its tests in parallel by default.
// Our file system has not been designed to support parallel accesses, and tests will hence fail unexpectedly if you try to run them in parallel.
// It is therefor very important to ensure that you run `cargo test` with the option `--test-threads=1`, or that you make sure that all of your tests are backed by a different disk.
// For the two tests below, we have taken the latter approach, for illustrative purposes.
// For more information, see [here](https://doc.rust-lang.org/book/ch11-02-running-tests.html#running-tests-in-parallel-or-consecutively).
//
//
// Note that one of the reason this assignment does not come with premade file systems, but rather, we set them up during the tests, is because the Serialization we will perform in higher layers of abstraction is platform-dependent, and binary formats might hence differ between different users.
#[cfg(test)]
mod tests {

    use super::Device;
    use crate::types::Block;
    use std::fs::{create_dir_all, remove_dir, remove_file};
    use std::path::{Path, PathBuf};

    // For these tests, we use a toy disk with 10 blocks, each containing 10 bytes
    static BLOCK_SIZE: u64 = 10;
    static NBBLOCKS: u64 = 10;

    //Returns the path to the image we will use during the tests
    //To avoid parallel tests from overlapping, each test also passes in its own unique `name` string, so it gets access to its own resources.
    //Also creates any missing directories between this path and the current working directory
    //Additionally, removes the "img10x10"-file, if it happens to exist already
    fn disk_prep_path(name: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("fs-images-controller-".to_string() + name);
        path.push("img");

        if path.exists() {
            //Remove the file in case it already exists
            remove_file(&path).unwrap();
        }
        {
            //Create any missing directories first, if applicable
            let prefix = path.parent().unwrap();
            create_dir_all(prefix).unwrap();
        }

        return path;
    }

    //Create a fresh 10x10 device
    fn disk_setup(path: &Path) -> Device {
        Device::new(path, BLOCK_SIZE, NBBLOCKS).unwrap()
    }

    //Create an existing 10x10 device
    fn disk_open(path: &Path) -> Device {
        Device::load(path, BLOCK_SIZE, NBBLOCKS).unwrap()
    }

    //Destruct the given device and remove the parent directory that is was located in
    fn disk_destruct(dev: Device) {
        let path = dev.path.to_owned();
        dev.destruct();
        remove_dir(path.parent().unwrap()).unwrap(); //Safety measure; will only delete an empty directory
    }

    // Now let's write our first test.
    //
    // Note that tests are annotated with `#[test]`, and cannot take arguments
    // nor return anything.
    //
    // Here we test some of the above methods on a fresh disk image, destroying it at the end.
    #[test]
    fn create_disk_test() {
        //Set up a new device
        let path = disk_prep_path("create");
        let mut dev = disk_setup(&path);

        //Check for some random blocks that they are indeed zero at start up
        let i1 = 3;
        let i2 = 9;
        let zero_block = |i| Block::new_zero(i, 10);
        let br = dev.read_block(i1).unwrap();
        assert_eq!(br, zero_block(i1));
        let br = dev.read_block(i2).unwrap();
        assert_eq!(br, zero_block(i2));

        //Read and write block 11; this should result in an error
        let ie = NBBLOCKS;
        assert!(dev.read_block(ie).is_err());
        assert!(dev.write_block(&zero_block(ie)).is_err());

        //Write blocks of the wrong size
        let sized_block = |s: u64| Block::new_zero(i1, s);
        assert!(dev.write_block(&sized_block(BLOCK_SIZE + 1)).is_err());
        assert!(dev.write_block(&sized_block(BLOCK_SIZE - 1)).is_err());

        //Read a vector containing numbers from 0->9,
        //and see if we read the same thing back
        let block_data = (0..10).collect();
        let bw = Block::new(i1, block_data);
        dev.write_block(&bw).unwrap();
        let br = dev.read_block(i1).unwrap();
        //Do we read what we wrote?
        assert_eq!(br, bw);

        //Write raw data and read it through the block interface
        let raw_data = &vec![1, 2, 3, 4, 5];
        dev.write(78, raw_data).unwrap(); //Write goes into blocks 7 and 8
        let br = dev.read_block(7).unwrap();
        let mut block_data = vec![0; 8];
        block_data.append(&mut vec![1, 2]);
        let bw = Block::new(7, block_data.into_boxed_slice());
        assert_eq!(br, bw);
        let br = dev.read_block(8).unwrap();
        let mut block_data = vec![3, 4, 5];
        block_data.append(&mut vec![0; 7]);
        let bw = Block::new(8, block_data.into_boxed_slice());
        assert_eq!(br, bw);

        //Read raw data that we wrote to i1 before
        let raw_data = dev.read(35, 5).unwrap();
        assert_eq!(raw_data, vec!(5, 6, 7, 8, 9).into_boxed_slice());

        disk_destruct(dev);
        //Make sure the file has actually been destroyed
        assert!(!path.exists());
    }

    // Here we test persistence of data after reloading a disk image, destroying it at the end.
    #[test]
    fn load_existing_disk_test() {
        //Set up a new device and make a few writes
        let path = disk_prep_path("load");
        let mut dev = disk_setup(&path);

        let i1 = 0;
        let i2 = 8;
        let block_data1 = (0..10).collect();
        let block_data2 = (0..10).rev().collect();
        let bw1 = Block::new(i1, block_data1);
        let bw2 = Block::new(i2, block_data2);
        dev.write_block(&bw1).unwrap();
        dev.write_block(&bw2).unwrap();

        //Close the device by dropping it
        drop(dev);

        //Reopen the device and assert that our old data is still there
        let dev = disk_open(&path);
        let br1 = dev.read_block(i1).unwrap();
        let br2 = dev.read_block(i2).unwrap();
        assert_eq!(br1, bw1);
        assert_eq!(br2, bw2);

        disk_destruct(dev);
        //Make sure the file has actually been destroyed
        assert!(!path.exists());
    }
}
