//! Module containing the types used in this project.
//! You can define your own wrappers around these types if you need more than the provided functionality.

use super::error_given;
use super::error_given::APIError;
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::prelude::*;
use std::io::{Cursor, SeekFrom};

/// Buffer abstraction, representing some data on the heap.
/// Buffers can have any size, and will be used further on to build file system `Block`s with, but also as output to read and write functions on files
/// Support regular read and write methods, but also (de)serialization of structures implementing the appropriate traits
#[derive(Debug, PartialEq, Eq)]
pub struct Buffer {
    ///Contents of the buffer, represented as a boxed slice
    /// The reason for this choice of data structure is that we will not have to change the size of buffers while using them.
    contents: Box<[u8]>,
}

impl Buffer {
    /// Create a new buffer, having the given `data` slice as its data
    pub fn new(data: Box<[u8]>) -> Buffer {
        Buffer { contents: data }
    }

    /// Create an all-zero buffer, with contents length of `len`
    pub fn new_zero(len: u64) -> Buffer {
        Buffer {
            contents: vec![0; len as usize].into_boxed_slice(),
        }
    }

    /// Size of the underlying block data
    pub fn len(&self) -> u64 {
        self.contents.len() as u64
    }

    /// Return a reference to this block's contents
    pub fn contents_as_ref(&self) -> &[u8] {
        return &self.contents;
    }

    /// Reads data from the given buffer into the `data` buffer, starting at the given `offset`.
    /// Returns the number of bytes that were read, or an error in case of failure.
    /// If the function does not return an error, the number of bytes read should always be equal to `data.len()`.
    pub fn read_data(&self, data: &mut [u8], offset: u64) -> error_given::Result<()> {
        if offset + data.len() as u64 > self.len() {
            return Err(APIError::BlockInput(
                "Trying to read beyond the bounds of the block",
            ));
        }

        let mut c = Cursor::new(&self.contents);
        c.seek(SeekFrom::Start(offset))?;
        c.read_exact(data).map_err(|e| e.into())
    }

    /// Writes data from the given slice into the `data.
    /// If the function does not return an error, the number of bytes written should always be equal to `data.len()`.
    pub fn write_data(&mut self, data: &[u8], offset: u64) -> error_given::Result<()> {
        if offset + data.len() as u64 > self.len() {
            return Err(APIError::BlockInput(
                "Trying to write beyond the bounds of the block",
            ));
        }

        let mut c = Cursor::new(&mut self.contents[..]);
        c.seek(SeekFrom::Start(offset))?;
        c.write_all(data).map_err(|e| e.into())
    }

    /// Read any object that implements the DeserializeOwned trait from this buffer
    ///
    /// *EXTRA*: Note that since this method takes ownership of the deserialized data, the link with the original data in the block necessarily breaks.
    /// This is not what you would have in a high-performance C implementation, as you would simply perform a cast of the part of memory you are interested in to a struct, without having to worry about lifetimes.
    /// To keep things simple and not have additional lifetime dependencies and unsafe code here, this method was not implemented as such.
    pub fn deserialize_from<S>(&self, offset: u64) -> error_given::Result<S>
    where
        S: DeserializeOwned,
    {
        let mut c = Cursor::new(&self.contents);
        c.seek(SeekFrom::Start(offset))?;
        Ok((bincode::deserialize_from(c))?)
    }

    /// Write any object that implements the Serialize trait into this buffer
    /// Goes through `write_data` so that the appropriate error get triggered.
    /// Alternatively, we could go through `serialize_into` [`bincode`](https://docs.rs/bincode/1.3.1/bincode/index.html) and use the standard error.
    pub fn serialize_into<S>(&mut self, stru: &S, offset: u64) -> error_given::Result<()>
    where
        S: Serialize,
    {
        let stru_bin = bincode::serialize(stru)?;
        //Going through write data so that the appropriate errors get triggered
        self.write_data(&stru_bin, offset)
    }
}

/// Block abstraction, representing a block of data read from the disk.
/// Provides basic methods to read and write data and select structures from and to a block.
/// The basic unit read and written by the device controller, that our file system will make use of.
///
/// *EXTRA*: Note that in a real life setting, the device controller would often simply read and write blocks from/to a memory-mapped region (this is a form of so-called DMA, Direct Memory Access).
/// This `Block` abstraction would hence not exist at the level of the controller.
/// We nevertheless lowered the abstraction of blocks to the level of the device controller, as it sending around raw `Vec`s or arrays is not much more realistic anyway.
#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    ///Index of this block sector on the disk
    pub block_no: u64,
    ///Contents of the block, represented as a `Buffer`. The block will relay all of its method implementations to this buffer contents.
    /// The reason for this choice of data structure is that we will not have to change the size of memory blocks while using them.
    buf: Buffer,
}

impl Block {
    /// Create a new block, corresponding to block `block_no` on disk, having the given `data` slice as its data
    pub fn new(block_no: u64, data: Box<[u8]>) -> Block {
        Block {
            block_no: block_no,
            buf: Buffer::new(data),
        }
    }

    /// Create an all-zero block, with contents length of `len`
    pub fn new_zero(block_no: u64, len: u64) -> Block {
        Block {
            block_no: block_no,
            buf: Buffer::new_zero(len),
        }
    }
    /// Size of the underlying block data
    pub fn len(&self) -> u64 {
        self.buf.contents.len() as u64
    }

    /// Return a reference to this block's contents
    pub fn contents_as_ref(&self) -> &[u8] {
        return &self.buf.contents_as_ref();
    }

    /// Reads data from the given block into the `data` buffer, starting at the given `offset`.
    /// Returns the number of bytes that were read, or an error in case of failure.
    /// If the function does not return an error, the number of bytes read should always be equal to `data.len()`.
    pub fn read_data(&self, data: &mut [u8], offset: u64) -> error_given::Result<()> {
        self.buf.read_data(data, offset)
    }

    /// Writes data from the given slice into the `data.
    /// If the function does not return an error, the number of bytes written should always be equal to `data.len()`.
    pub fn write_data(&mut self, data: &[u8], offset: u64) -> error_given::Result<()> {
        self.buf.write_data(data, offset)
    }

    /// Read any object that implements the DeserializeOwned trait from this block
    ///
    /// *EXTRA*: Note that since this method takes ownership of the deserialized data, the link with the original data in the block necessarily breaks.
    /// This is not what you would have in a high-performance C implementation, as you would simply perform a cast of the part of memory you are interested in to a struct, without having to worry about lifetimes.
    /// To keep things simple and not have additional lifetime dependencies here, this method was not implemented as such.
    pub fn deserialize_from<S>(&self, offset: u64) -> error_given::Result<S>
    where
        S: DeserializeOwned,
    {
        self.buf.deserialize_from(offset)
    }

    /// Write any object that implements the Serialize trait into this block
    /// Goes through `write_data` so that the appropriate errors get triggered, because using `serialize_into` from [`bincode`](https://docs.rs/bincode/1.3.1/bincode/index.html) would risk extending the underlying vector instead of throwing an error.
    pub fn serialize_into<S>(&mut self, stru: &S, offset: u64) -> error_given::Result<()>
    where
        S: Serialize,
    {
        self.buf.serialize_into(stru, offset)
    }
}

/// Structure representing all file system metadata that we are interested in, and hence the file system's structure.
/// Note that the size of the Superblock struct does not necessarily have to be a full block, as it can just be read from disk contiguously.
/// Rather, the size of `SuperBlock` must be at most as large as a single disk block.
/// Derives `Serialize` and `Deserialize` so we can easily write this block to the disk and read it again after.
///
/// The layout of the simple file system model we use is as follows:
///     \[super block | inode blocks | free bit map | data blocks\]
/// , where each component has the following meaning:
///
/// 1. *super block*: aggregates all the file system meta-data including the sizes of all subsequent regions. This is the first block that is read by the file system driver when loading an existing file system, and the first block to be written by the driver in case a new file system is initialized. This area should consist of a single block, i.e. the `SuperBlock` type defined below should not take up more space in memory than a single block, defined by `Disk.block_size` in [`controller.rs`](../controller/index.html).
/// 2. *inode blocks*: a sequence of blocks containing all the inode metadata. This region contains all inodes in order, starting from inode 1 (the root directory, i.e. the directory on your computer with path "\"), all the way up to the last inode. The number of inodes stored in each block is equal to the floor of the block size divided by the inode size, i.e. blocks are packed with inodes, and individual inodes are always entirely stored in a single block (they are never broken up over multiple blocks).
/// 3. *free bit map*: a sequence of blocks keeping track of the allocation state (allocated or free) of all disk blocks in the next data block region. The *n*th bit in this sequence specifies whether or not the *n*th data block is currently in use.
/// 4. *data blocks*: contain the actual file and directory data, as a long sequence of disk blocks.
///
/// *EXTRA*: Since we do not support logging, there is no need for an additional memory region to store any logs in
/// Also note that in contrast to more realistic device layouts, we ignore the fact that the first block of the device is often reserved for bootstrapping code, and makes use of e.g. a Master Boot Record (MBR) or Volume Boot Record (VBR).
/// *EXTRA*: Note that just like blocks, inodes are not being cached either. The consequence is that the users of our APIs are responsible for ensuring that they aren't handling different aliases to the same inode without realizing it. This will not scale well to a parallellized setting. In our case, this is no major problem, as we have no parallellism, and we have simple system call interactions, that will not handle a lot of inodes at the same time, and will hence not need to perform many of those inode equality checks.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SuperBlock {
    ///Size of the blocks in the current file system, in *BYTES*\
    ///In a real world application, this block size does not necessarily match the size of the sectors on the device itself, but for simplicity reasons we assume this value and the disk sector size in `Device.block_size` to always be equal
    pub block_size: u64,
    ///Number of blocks in the entire file system, including this block and the 3 other file system regions\
    ///This number does not necessarily have to equal the size of the disk this file system is stored on, as long as the file system fits on the disk
    pub nblocks: u64,
    ///Number of inodes that we keep track of in the inode region\
    ///This number does not necessarily have to fill up the entire region, i.e. it is possible to make the inode region unnecessarily big
    pub ninodes: u64,
    ///The block index of the first block of inodes\
    ///Since the super block is only a single block long and located at index 0, this will usually be the block with index\
    ///The inode region runs until `bmapstart`\
    ///The inode region is assumed to be sufficiently long to contain `niondes` inodes
    pub inodestart: u64,
    ///Number of data blocks that we keep track of in the bitmap region\
    ///This number does not necessarily have to fill up the entire bitmap or data region, i.e. it is possible to make the bitmap and/or block data region unnecessarily big
    pub ndatablocks: u64,
    ///The block index of the first block of the free bit map region\
    ///The free bit map region runs until `datastart`\
    ///The free bit map region is assumed to be at least `ndatablocks` bits large\
    pub bmapstart: u64,
    ///The block index of the first block of the data blocks region\
    ///The data block region runs until `nblocks`, i.e. the end of the file system\
    ///The data block region is assumed to be at least `ndatablocks` blocks large
    pub datastart: u64,
}

lazy_static! {
    /// Size the superblock takes up in memory on your system, in bytes.
    /// This size can only be found out at runtime, which is the reason why we have to wrap this code in a `lazy_static` macro.
    /// Notice the use of the `ref` keyword; `SUPERBLOCK_SIZE` is a reference to an `u64` number, that will only be filled in at runtime.
    /// Used to determine the number of inodes per block, which is important for filesystem initialization.
    pub static ref SUPERBLOCK_SIZE : u64 = bincode::serialize(&SuperBlock::default()).unwrap().len() as u64;
}

/// Hard-coded number of data blocks each inode can point to
pub const DIRECT_POINTERS: u64 = 12;

/// Enum describing file types
/// Currently, either a file `T_FILE`, a directory `T_DIR` or a free inode `T_Free`
/// The file type `T_FREE` is used to signify a free inode, that can be used to allocate a new file or directory.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Copy, Clone)]
pub enum FType {
    /// Directory file type
    TDir,
    /// Regular file type
    TFile,
    /// Free file type
    TFree,
}
impl Default for FType {
    fn default() -> FType {
        FType::TFree
    }
}

/// Struct describing data held by an inode on the disk.
/// Derives the Serialize and Deserialize traits, to allow for easy (de-)serialization when writing to disk blocks
///
/// *EXTRA*: In real-life file systems, files also contain a field pointing to a data block containing more data blocks, called an indirect pointer.
/// For simplicity reasons, we do not support this in the current file system.
/// In other words, files are made up of a total of at most `DIRECT_POINTERS` data blocks.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct DInode {
    /// Registers the file type
    pub ft: FType,
    /// Counts the number of links to this inode in the file system. The point of doing this is that if the inode is written back to disk when it has no links to it, it should be freed instead, thereby setting its file type to `T_FREE`.
    pub nlink: u16,
    /// Size of the file in bytes. Used to see when a read or write would go out of file bounds.
    pub size: u64,
    /// A list of up to `DIRECT_POINTERS` valid data block addresses, to specify where the contents of this file are stored.
    pub direct_blocks: [u64; DIRECT_POINTERS as usize],
}

lazy_static! {
    /// Size of an inode in your system, in bytes.
    /// This size can only be found out at runtime, which is the reason why we have to wrap this code in a `lazy_static` macro.
    /// Notice the use of the `ref` keyword; `DINODE_SIZE` is a reference to an `u64` number, that will only be filled in at runtime.
    /// Used to determine the number of inodes per block, which is important for filesystem initialization.
    pub static ref DINODE_SIZE : u64 = bincode::serialize(&DInode::default()).unwrap().len() as u64;
}

/// Inode number of the root inode
pub const ROOT_INUM: u64 = 1;

/// Wrapper around disk inodes `DInode` used for in-memory inodes.
/// Additionally contains the number of the inode `inum`.
/// This information is not required as long as the inode is stored on disk, as it is implicit from the block in which the inode is stored.
/// This is analogous to a [`Block`](../block/struct.Block.html) explicitly keeping track of its block number
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Inode {
    /// inode number
    pub inum: u64,
    /// the disk contents corresponding to `inum`
    pub disk_node: DInode,
}

impl Inode {
    /// Create a new inode
    pub fn new(inum: u64, disk_node: DInode) -> Inode {
        Inode { inum, disk_node }
    }
}

/// Trait for inode-like behavior, so that we can have different inodes later on, without having to change the interfaces of the inode trait
/// Solely used in testing, so does not require setter methods
pub trait InodeLike: Sized {
    ///Create a new inode from the given parameters
    ///The user of this method is responsible for making sure that the parameters are consistent with each other and the rest of the code
    fn new(inum: u64, ft: &FType, nlink: u64, size: u64, blocks: &[u64]) -> Option<Self>;
    ///Get the file type of this inode
    fn get_ft(&self) -> FType;
    ///Get the number of links to this inode in the file system
    fn get_nlink(&self) -> u64;
    ///Get the size of this inode in bytes
    fn get_size(&self) -> u64;
    ///Get the address of the *i*th block pointed to by this inode, if there is any block under index *i*
    ///Note that this function's behavior is undefined for unallocated indexes `i` (as these are commonly set to 0). It is up to the caller to figure out whether the index *i* was sensible, based on `get_size()` and the external file system block size.
    fn get_block(&self, i: u64) -> u64;
    ///Get the number of this inode on the disk
    fn get_inum(&self) -> u64;
}

///You get the implementation of `InodeLike` for free for the `Inode` I defined above
///You will have to write different implemenations of this trait in case you complete the extra assignments.
impl InodeLike for Inode {
    fn new(inum: u64, ft: &FType, nlink: u64, size: u64, blocks: &[u64]) -> Option<Self> {
        if nlink > u16::MAX as u64 {
            return None;
        }
        if blocks.len() > DIRECT_POINTERS as usize {
            return None;
        }

        let mut db = [0; DIRECT_POINTERS as usize];
        for i in 0..blocks.len() {
            db[i] = blocks[i];
        }

        let di = DInode {
            ft: *ft,
            nlink: nlink as u16,
            size,
            direct_blocks: db,
        };
        Some(Inode::new(inum, di))
    }

    fn get_ft(&self) -> FType {
        self.disk_node.ft
    }
    fn get_nlink(&self) -> u64 {
        self.disk_node.nlink as u64
    }
    fn get_size(&self) -> u64 {
        self.disk_node.size
    }
    fn get_block(&self, i: u64) -> u64 {
        if DIRECT_POINTERS <= i {
            return 0;
        }
        self.disk_node.direct_blocks[i as usize]
    }

    fn get_inum(&self) -> u64 {
        self.inum
    }
}

/// Hard-coded number of characters each directory entry can contain for its name
pub const DIRNAME_SIZE: usize = 14;

/// Specific type of inode contents for directories
/// A directory is a file containing a sequence of DirEntry structures, with the `FType` set to the directory type `TDir`.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct DirEntry {
    ///Number of the inode this directory entry points to
    ///It is these types of pointers that cause inode's `nlink` fields to increase in the file system
    ///A directory entry with an `inum` of 0 represents an empty entry
    pub inum: u64,
    ///Character array specifying the name of this entry\
    ///Names can be up to `DIRNAME_SIZE` characters long\
    ///Shorter names can be specified by storing the null termination character `\0` inside the array; this character indicates the end of the name string
    ///Note that `char` in Rust is UTF-8 encoded and always takes up 4 bytes. This saves us headaches in the conversion below, at the cost of some memory efficiency
    pub name: [char; DIRNAME_SIZE],
}

lazy_static! {
    /// Size of an directory entry in your system, in bytes.
    /// For similar reasons, again wrapped in the `lazy_static!` macro.
    pub static ref DIRENTRY_SIZE : u64 = bincode::serialize(&DirEntry::default()).unwrap().len() as u64;
}

///Tests for the block type
#[cfg(test)]
mod block_tests {

    use super::Block;
    use serde::{Deserialize, Serialize};

    // For these tests, we use blocks containing 1000 bytes, which should be enough to store a few inodes
    static BLOCK_SIZE: u64 = 1000;

    //Testing the raw read/write methods offered by blocks
    #[test]
    fn raw_rw_test() {
        let n1 = 12;
        let mut b1 = Block::new_zero(n1, BLOCK_SIZE);
        //Contents have been installed correctly
        assert_eq!(b1.contents_as_ref(), vec![0; BLOCK_SIZE as usize]);

        //Write and then reread some raw data
        let mut raw_data = vec![5; 5];
        b1.write_data(&raw_data, 10).unwrap();
        b1.read_data(&mut raw_data, 8).unwrap();
        assert_eq!(raw_data, vec!(0, 0, 5, 5, 5));

        //Try to read or write out of bounds
        //Trivial out of bounds
        let mut emp = vec![];
        assert!(b1.write_data(&emp, BLOCK_SIZE).is_ok()); //double check
        assert!(b1.write_data(&emp, BLOCK_SIZE + 1).is_err());
        assert!(b1.read_data(&mut emp, BLOCK_SIZE + 1).is_err());
        //Read or write out of bounds because of buffer size
        let mut one = vec![1];
        assert!(b1.write_data(&one, BLOCK_SIZE).is_err());
        assert!(b1.read_data(&mut one, BLOCK_SIZE).is_err());
        let mut two = vec![1, 2];
        assert!(b1.write_data(&two, BLOCK_SIZE - 1).is_err());
        assert!(b1.read_data(&mut two, BLOCK_SIZE - 1).is_err());
    }

    //Importing some example deserializable struct
    use crate::types::{DInode, FType, DINODE_SIZE, DIRECT_POINTERS};
    //Another testing struct to perform (de)serialization on
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct Point(u64, u64);

    //Testing the (de)serialization methods offered by blocks
    #[test]
    fn serialization_test() {
        let p1 = Point(0, 0);
        let p2 = Point(1000, 1000);
        let in1 = DInode::default();
        let in2 = DInode {
            ft: FType::TFree,
            nlink: 13,
            size: 142,
            direct_blocks: [1000; DIRECT_POINTERS as usize],
        };

        //Testing some length consistency, and the global variable DINODE_SIZE
        assert_eq!(
            bincode::serialize(&p1).unwrap().len(),
            bincode::serialize(&p1).unwrap().len()
        );
        assert_eq!(
            bincode::serialize(&in1).unwrap().len(),
            *DINODE_SIZE as usize
        );
        assert_eq!(
            bincode::serialize(&in1).unwrap().len(),
            bincode::serialize(&in2).unwrap().len()
        );

        let n1 = 12;
        let mut b1 = Block::new(n1, vec![1; BLOCK_SIZE as usize].into_boxed_slice());
        let point_size = bincode::serialize(&p1).unwrap().len() as u64;
        //Now perform some actual writes to the block, and read them again after
        b1.serialize_into(&p1, 0).unwrap();
        b1.serialize_into(&p2, point_size).unwrap();
        b1.serialize_into(&in1, 2 * point_size).unwrap();
        b1.serialize_into(&in2, 2 * point_size + *DINODE_SIZE)
            .unwrap();
        assert_eq!(b1.deserialize_from::<Point>(0).unwrap(), p1);
        assert_eq!(b1.deserialize_from::<Point>(point_size).unwrap(), p2);
        assert_eq!(b1.deserialize_from::<DInode>(2 * point_size).unwrap(), in1);
        assert_eq!(
            b1.deserialize_from::<DInode>(2 * point_size + *DINODE_SIZE)
                .unwrap(),
            in2
        );

        //Perform reads and writes that go out of bounds
        let mut b1 = Block::new_zero(n1, BLOCK_SIZE);
        assert!(b1
            .deserialize_from::<Point>(BLOCK_SIZE + 1 - point_size)
            .is_err());
        assert!(b1.serialize_into(&p2, BLOCK_SIZE + 1 - point_size).is_err());
        //Ensure contents don't change after faulty reads or writes
        assert_eq!(b1.contents_as_ref(), vec![0; BLOCK_SIZE as usize]);
    }
}
