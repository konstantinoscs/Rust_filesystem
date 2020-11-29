//! Collection of all traits you will be implementing throughout the different assignments
//! Each file system must at least implement the `FileSysSupport` trait.
//! Do not just blindly implement the below traits; in most cases, it is a good idea to define auxiliary functions, to e.g. figure out in what block inode $i$ is stored, how many directory entries fit in a block, or to read a specific byte from a block.
//! Make sure your implementation provides auxiliary functions for repetitive tasks like these.
//! You might need to wrap (some of) the types I provided in the API into your own types, to be able to define additional behavior on them.

use super::{
    controller::Device,
    types::{Block, Buffer, DirEntry, FType, InodeLike, SuperBlock},
};
use std::{error, path::Path};

/// General trait that each filesystem should implement, that allows us to set up, tear down and load file systems in the tests
/// Additionally, this trait also defines the error type that is used in all of the other traits (which will require implementing this trait)
/// Be warned that the implementation of this trait cannot be kept the same throughout the assignment!
/// Depending on the abstractions your current file system is aware of, it might impose more strict conditions when loading a file system, and might perform extra steps when defining one.
/// For example, when implementing the block layer for your file system, it should not create a root inode as part of `mkfs`, and neither should it check for one when mounting an existing file system. These steps should only be implemented at higher levels of abstraction.
/// In other words, you should *not* have one blanket implementation of this trait for your entire project, and it will be beneficial to you when e.g. defining the inode abstraction, to take your solution from the block layer, including its implementation of this trait, and create a wrapper around it that (re)implements this and any other relevant traits, so that you can delegate calls to this trait to the wrapped object, and define extra steps on top of them.
///
/// Consult the documentation of [`SuperBlock`](../types/struct.SuperBlock.html) for more detailed information on the proper layout of the file system and the meaning of the different superblock components
///
/// *HINT*: consider caching your filesystem superblock when creating or loading a file system, so you do not have to read it each time you need it
pub trait FileSysSupport: Sized {
    /// The type of the errors of your implementation.
    /// Read more about how to define your own errors in the documentation of [`error_given`](../error_given/index.html).
    ///
    /// Note the “supertrait” your error type must implement:
    /// [`error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
    /// This is the base trait for all Rust errors, click
    /// on it to see the methods you must implement.
    ///
    /// Note that you can use the same error type for multiple parts of the assignment.
    type Error: error::Error;

    /// Static method to check if a given superblock represents a valid file system.
    /// You will need this both when creating a new file system, and loading an existing one from disk
    ///
    /// More concretely, the following conditions need to hold to make a `SuperBlock` a possibly valid representation of a file system
    /// - the regions have to appear in the right order
    /// - the regions have to be sufficiently large to hold `ninodes` inodes (calculated using the size of your inodes) and to hold and keep track of `ndatablocks` datablocks
    /// - the regions have to physically fit on the disk together, i.e. fall within the first `nblocks` blocks
    fn sb_valid(sb: &SuperBlock) -> bool;

    /// Method to create and mount a new file system from scratch, given a super block (which is a bit more convenient to work with than a bunch of parameters) and a path.
    /// What exactly this method does, depends on the level of abstraction you are implementing.
    ///
    /// This method always does the following, regardless of the layer of abstraction:
    /// - Check if the given superblock is a valid file system superblock
    /// - Create a new `Device` at the given path, to allow the file system to communicate with it
    ///
    /// Then, subdivide the freshly created device image into the previously described regions:
    /// 1. A super block containing the file system metadata at block index 0
    /// 2. *This is only relevant for the inode layer and up, i.e. ignore this set-up step in the disk block layer.*
    ///    A list of blocks containing inodes, where all inodes as marked as "free" (since they are free, the rest of their contents is irrelevant and hence unspecified).\
    /// *You only need to do the following once you support directories:*
    /// The first inode (with number 1) describes the "root path", i.e. the path "/" on UNIX systems. It is initially empty, i.e. has no directory entries inside it.
    /// The root node has its `nlink` field set to 1 from the start, even though there are no references to it, so that it cannot be deallocated.
    /// Note that inodes start counting at one, and *NOT* at zero, to avoid confusion with the error return value 0 in the kernel.
    /// However, to avoid off-by-1 errors, room for inode 0 is still allocated in the first inode block on disk (i.e. inode 1 is *not* stored at address 0 of this block).
    /// This space will under normal circumstances never be used.
    /// 3. A bitmap keeping track of the occupied memory blocks.
    /// This bitmap should initially mark all blocks as "free", as no block allocations have happened.
    /// 4. A data region to contain the memory blocks themselves
    /// Since all blocks are marked as free after initialization and allocating a block should set its contents to 0, the contents of this region is unimportant.
    /// This data region is assumed to run until the end of the file system.
    ///
    /// Think about whether the initial `Device` you create inside this method, which will contain 0 at each address (see the [documentation](../controller/struct.Device.html#method.new)), is a valid disk representation, or if it requires extra initialization for one or more of the above regions.
    /// (*Hint for the inode layer and up*: watch out for the inodes; an all-0 inode will not necessarily come out well during deserialization, and probably needs to be overwritten by an actually free inode)
    ///
    /// Make sure your underlying device is in a consistent state and matches the above enumeration at the end of this function.
    ///
    /// *IMPORTANT NOTE*: In case you need to loop over inodes here or anywhere else in this project, do so **efficiently**, i.e. if you need to read/write multiple inodes in the same block, only load and store this block once!
    ///
    /// *EXTRA*: mkfs is inspired by the unix command of the same name (although this version also immediately mounts the file system)
    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error>;

    /// Given an existing `Device` called `dev`, make sure that its image corresponds to a valid file system by reading its superblock and checking the following conditions:
    /// - The superblock is a valid superblock
    /// - The block size and number of blocks of the device and superblock agree
    ///
    /// If these conditions are satisfied, wrap the given `Device` in a file system and return it.
    ///
    /// You do **not** need to deserialize each individual object in each region to check that it is indeed a valid object; to keep matters simple, we will assume that the contents of each region has been properly initialized.
    /// Additionally, we could add `dev` to the return type to reclaim ownership in case of an error, but we do not bother recovering invalid devices, for simplicity reasons.
    fn mountfs(dev: Device) -> Result<Self, Self::Error>;

    /// Unmount the give file system, thereby consuming it
    /// Returns the image of the file system, i.e. the `Device` backing it.
    /// The implementation of this method should be almost trivial
    fn unmountfs(self) -> Device;
}

/// This trait adds block-level operations to your file system
/// Do not forget to make sure that `mkfs` now correctly initializes all inodes, as described above.
pub trait BlockSupport: FileSysSupport {
    /// Read the *n*th block *of the entire disk* and return it\
    /// The implementation of this method should be trivial
    fn b_get(&self, i: u64) -> Result<Block, Self::Error>;

    /// Write the *n*th block *of the entire disk* and return it\
    /// The implementation of this method should be trivial
    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error>;

    /// Free the *i*th block *in the block data region*, by setting the *i*th bit in the free bit map region to zero.
    /// To disambiguate the binary representation, we define the order of the bits within each byte you read from right to left, i.e. the byte '0b0000_0001' has its **first** bit set to one, i.e. has its bit with index 0 set to 1.
    /// Be careful not to change any bits for other blocks around the *i*th block by overwriting an entire byte in the free bit map region!
    /// Consider writing some auxiliary methods for more fine-grained access to `Block`s, by wrapping the provided block type in a different type.
    ///
    /// Remember that you *cannot* access memory on a per-bit basis!
    /// The bitwise operators `&` (bitwise AND), `|` (bitwise OR), `^` (bitwise XOR) exist; use those, together with the bit-shift operators `>>` and `<<`, to change the correct bit (or use an external, stable crate).
    ///
    /// If the *i*th block is already a free block, the state of memory should not change, and this method should return an error
    /// Also errors if the index *i* is out of bounds, i.e. if it is higher than the number of data blocks
    fn b_free(&mut self, i: u64) -> Result<(), Self::Error>;

    /// Zeroes the *i*th block *in the block data region* in memory (without freeing it)\
    /// Errors if the index *i* is out of bounds, i.e. if it is higher than the number of data blocks
    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error>;

    /// Allocate the first free block, starting from the beginning, in the data block region, thereby setting its bit in the bitmap region to one and the entire contents of the block to zero.
    /// Again, you will need bit-wise operators to implement this function.
    /// Obviously, only blocks that have not been allocated yet, i.e. do not have their bit set, can still be allocated.
    /// Returns the index (*within the data region*) of the newly allocated block.
    /// Make sure to load each *bitmap* block only once in your implementation.
    /// Errors appropriately if no blocks are available.
    fn b_alloc(&mut self) -> Result<u64, Self::Error>;

    /// Get the superblock describing the current file system
    fn sup_get(&self) -> Result<SuperBlock, Self::Error>;

    /// Write the superblock to this file system's underlying device (and cache it, depending on how you implement your file system)
    /// Note that in general, when you write to a block, you should read the block first and then overwrite the parts you need to change before writing it back, to make sure all other parts of the block remain unchanged.
    /// In this case, that is not strictly necessary, as the superblock is the only useful thing that is stored on the first disk block.
    /// However, in case other data were to be stored past the superblock struct in the future, do implement this function in this conservative way.
    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error>;
}

/// This trait adds the abstraction of inodes to your file system.
/// Do not forget that `mkfs` should now result in a filesystem where each inode is marked as free.
/// Note that this trait does not yet support ways of growing inodes.
/// Growing inodes will become possible when you implement the read and write calls on them in one of the optional assignments.
pub trait InodeSupport: BlockSupport {
    /// The type you use for your inodes at runtime. Initially, you can probably get away with [the inode type I provided](../inode/struct.Inode.html).
    /// In the extensions, we can replace this type with a different type of inode that provides more flexibility (e.g. aliasing behavior, and a variable number of blocks)
    type Inode: InodeLike;

    /// Read the disk inode with index `i` and wrap it into an inode object.
    /// Should error if `i` is greater than the number of inodes in the system.
    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error>;

    /// Write the given inode back to the disk at the correct position
    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error>;

    /// Similar to b_free, but now for inodes instead of data blocks.
    /// A big difference is that this method should only free an inode if it is no longer referenced anywhere else in the file system, i.e. if its `nlink` field is equal to zero.
    /// In this case, the $i$th inode in the inode region is freed by setting its `ft` field to `TFree`.
    /// Additionally, all `size` valid blocks belonging to this file, listed in its `direct_blocks` array, have to be deallocated (and set to address 0) as well.
    /// In case `nlink` is not equal to zero, this method does nothing.
    /// Returns an error if `i` does not correspond to an inode number, or if the inode is already free.
    fn i_free(&mut self, i: u64) -> Result<(), Self::Error>;

    /// Similar to b_alloc, but now for inodes instead of data
    /// Allocates the first free `dinode` (i.e. lowest `inum`) it comes across, and sets (on disk):
    ///  - this inode's `FType` to `ft`
    ///  - this inode's `size` and `nlink` fields to 0, as it currently has no blocks and is not referenced in the file system
    /// The inode with index 0 should *never* be allocated.
    /// Errors appropriately if no inodes are available
    /// Only read each inode block once in your implementation
    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error>;

    /// Truncate the given `inode`, i.e. release its contents (without freeing it).
    /// Sets all of the given inode's `direct_blocks` to point to address 0.
    /// Releases all blocks `direct_blocks` belonging to this inode, and sets its `size` to 0.
    /// Changes both the given `inode` and the corresponding inode on the disk.
    /// Note that only the first `size` blocks should be released as only these are allocated. In other words, do not blindly release all values listed in the `direct_blocks` field
    fn i_trunc(&mut self, inode: &mut Self::Inode) -> Result<(), Self::Error>;
}

///This trait additionally provides support to read and write from inodes using buffers; the data structure that we used before to hold the contents of a `Block`.
///The reason we use `Buffer`s objects and not simply the raw data, is because `Buffer`s also support some methods relating to (de)serialization, which might come in handy in some situations.
/// We ignore arithmetic over- and underflow in the following functions, for reasons of simplicity.
pub trait InodeRWSupport: InodeSupport {
    /// Read `n` bytes of data from inode `inode` into the given buffer `buf`, starting from byte offset `off` in the inode `inode`
    /// Returns the amount of bytes read, so that clients of this function can check if something went wrong
    /// If the end of the file is reached while reading, stop reading.
    /// If a read starts at `inode.get_size()`, returns with 0 bytes read.
    /// By contrast, returns an error and does not read anything in case the provided index falls further outside of the file's bounds.
    ///If `buf` cannot hold `n` bytes of data, reads until `buf` is full instead.
    fn i_read(
        &self,
        inode: &Self::Inode,
        buf: &mut Buffer,
        off: u64,
        n: u64,
    ) -> Result<u64, Self::Error>;

    /// Write `n` bytes of data from the given buffer `buff` into the inode `inode`, starting from byte offset `off`
    /// If the end of the file is reached while writing, **continue writing**.
    /// If necessary, start allocating extra blocks to expand the file and continue writing into the new blocks.
    /// Allows writes to start at index `inode.get_size()`.
    /// By contrast, returns an error and does not write anything in case the provided starting index falls further outside of the file's bounds.
    /// If the inode changes while writing, do not forget to write it back to the disk too.
    /// Returns an error if `buf` cannot hold at least `n` bytes of data.
    /// If the write would make the inode exceed its maximum possible size, do nothing and return an error.
    fn i_write(
        &mut self,
        inode: &mut Self::Inode,
        buf: &Buffer,
        off: u64,
        n: u64,
    ) -> Result<(), Self::Error>;
}

///This trait adds the abstraction of directories and their entries to the file system
/// Additionally, it supports some convenience methods that allow you to use directory entries with string names (the reason these methods are defined here and not in a trait, is to avoid forcing you to wrap the `DirEntry` type in another type of your own).
///Do not forget to make sure that `mkfs` now defines a valid (currently empty) root directory.
///
///Note that `i_free` is possibly unsafe for directories; if the freed directory is the last one to point to some entry and we free the directory's inode, the entry will be left dangling.
///To combat this, system calls will never call `i_free` directly, but rather call wrapper methods such as e.g. `unlink` (see the [`PathSupport`] trait and optional assignment for more information), to check all the necessary preconditions.
///The standard directory entries of ".." and ".", that are present in any directory but the parent directory, will be implemented as part of the [`PathSupport`] extension too.
///
/// [`PathSupport`]: ../fs/trait.PathSupport.html
pub trait DirectorySupport: InodeSupport {
    /// Create a new directory entry, given `inum` and `name`
    /// Returns `None` if an invalid name is provided
    /// Uses `set_name_str` to set the directory entry name
    fn new_de(inum: u64, name: &str) -> Option<DirEntry>;

    /// Get the name of this directory entry as a `String`
    /// Loop over the characters in the direntry's name until you encounter a final character `\0` OR until you reach the end of the character array, and return the concatenation of these characters as the `Direntry`'s name.
    fn get_name_str(de: &DirEntry) -> String;

    /// Set the name of this directory entry to the given `name`, if the given name is valid, i.e. it is
    ///- non-empty
    ///- consists of alphanumeric characters only, or is equal to "." or ".."
    ///- is sufficiently short when converted to characters
    /// If the `name` is shorter than `DIRNAME_SIZE`, insert a '\0' at the end so you can still correctly read it after.
    /// Returns `None` in case of an invalid name
    fn set_name_str(de: &mut DirEntry, name: &str) -> Option<()>;

    /// Look for a directory entry named `name` in a given inode, representing a directory.
    ///Make sure that the `inode` you are calling this function with is up to date wrt. the one on disk!
    /// If found, return the `inode` corresponding to this directory entry, and the byte offset (from the start of the inode contents) it was found at, as a pair.
    /// Only inspects directory entries that fall within the `size` of the given `inode`. Errors if the given `inode` is not of directory type.
    /// If the entry is not found, return a suitable error that you can handle later, in the implementation of `dirlink` below.
    ///
    /// NOTE: self is mutably borrowed here because might want to call iget
    fn dirlookup(&self, inode: &Self::Inode, name: &str)
        -> Result<(Self::Inode, u64), Self::Error>;

    /// Write a new directory entry with contents `name` and `inum` into the directory represented by `inode`.
    ///Make sure that the `inode` you are calling this function with is up to date wrt. the one on disk!
    /// In case the directory has no free entries, append a new entry to the end and increase the size of `inode`. This might require allocating a new block as well.
    /// In case you implemented the optional `InodeRWSupport` assignment already , you could use the `i_write` method here.
    ///
    /// When a place for the given `inode` is found, looks up the inode corresponding to `inum` and increase its `nlink` field with 1 on disk (unless `inum` and `inode`'s number are equal, then nothing happens, as this is a self-reference).
    /// Errors if
    /// - `name` is invalid, or is already an entry inside `inode`.
    /// - `inode` is not a directory.
    /// - errors *and does nothing* if the inode corresponding to `inum` is not currently in use.
    ///
    /// Returns the byte offset at which the entry was written into the given `inode`
    ///
    /// *EXTRA*:In our model, we do not have to worry about hardlink-loops in our file system tree, as we currently provide no way of duplicating inodes.
    fn dirlink(
        &mut self,
        inode: &mut Self::Inode,
        name: &str,
        inum: u64,
    ) -> Result<u64, Self::Error>;
}

///Enhance the previous directory support with a notion of file paths (both absolute and relative), enabling the following:
///- Allows looking up inodes along file paths, to allow for easier navigation.
///- Allows creating and unlinking (i.e. removing) directories at a given path location. The previously implemented `i_alloc` and `i_free` suffice for regular files, but can result in inconsistencies for directories.
///- Keeps track of the current working directory in your file system (pick an appropriate type to do this! **WARNING: give this some thought **). Usually, each process has its own current working directory, but since we only model a single process, we just store the current working directory in the file system.
///
///Do not forget to adapt the call to `mkfs`, so that the root directory is no longer empty, but like all other directories, gets initialized with "." and ".." as its first two entries. The root directory is a special case, however, in the sense that ".." also points back to the root directory itself, since it has no parent.
///Make sure the `nlink` field of the root directory is equal to 1 still, after the initialization phase.
///Additionally, both the `mkfs` and `mountfs` calls initialize the current working directory to the root directory.
///
///**IMPORTANT**
/// for this assignment;
/// - We assume that none of the directories involved in the cwd's path will be deleted while we are in this directory. We do not check this at any point, nor do we lock any of those files or increase their `nlink`; we just assume that it will not happen. Additionally, we do not check that the cwd actually also exists in the file system; this is the responsibility of our clients
/// - In general, it is **not** necessarily the case the parent of a directory is the previous directory in the path!
/// For example, starting from `/test` then following the path `../alternative` by reading the file system, does not necessarily have us end up in `/alternative`. The reason is that the parent of `test` might be some different directory entirely, because the `dirlink` method below allows us to re-register inodes in different directories.
/// **However**, you can assume that the parent of the cwd **is** the previous directory in the path when appending a relative path in the `set_cwd` method. This is also how terminals usually operate when e.g. following symbolic links and then running `cd ..`, in order not to confuse users.
/// That being said, any paths provided to other methods than `set_cwd` **should go through the file system**, i.e. read ".." at each point to figure out what the actual parent inode is.
/// If this explanation is unclear to you, consult the tests provided with the assignment
pub trait PathSupport: DirectorySupport {
    /// Returns true iff the given string represents a valid path
    /// We support two different path formats:
    /// - *absolute paths* start with the character "/" and specify a path starting from the root node
    /// - *relative paths* start with "." or ".." and denote respectively a path starting from the current working directory, and a path starting from the parent of the current working directory
    ///
    /// A path is valid iff:
    /// - It is not empty
    /// - It is either absolute or relative in form
    /// - It consists of a "/"-separated sequence of *names*
    /// - It does not end in a "/" (the only exception being the path "/" itself)
    /// - Each one of these names is a *valid* directory entry name (see the [`DirectorySupport`](../fs/trait.DirectorySupport.html) trait)
    /// - Instead of an alphanumeric name, the special entries "." and ".." can also appear anywhere in the file path. Their meaning is then similar to their meaning they have at the start of a relative path; "." specifies staying in the current directory, whereas ".." specifies moving up to the parent directory.
    fn valid_path(path: &str) -> bool;

    ///Return the current working directory as a String path
    ///The value you return here should be an absolute path with respect to the root path `/`, and should not contain the special names "." or ".."
    fn get_cwd(&self) -> String;

    ///Set the current working directory to the path provided in the given String, if it is a valid path.
    /// Returns none if the provided path was invalid.
    ///
    /// If a relative path is provided, then that path is interpreted with respect to the current working directory.
    /// As stated above, you should then **not** go through the file system to determine the new cwd, but rather have ".." cancel out the previous directory name, i.e. if the cwd is `test/child` then providing `../child_new` to this method, will unconditionally result in a new cwd of `test/child_new`, whereas it will not necessarily lead there when we read through the file system in the other methods.
    fn set_cwd(&mut self, path: &str) -> Option<()>;

    ///Given a path name (possibly relative to the cwd), look up the inode corresponding to this path (the final inode could be either a file or a directory), and return it.
    ///Works as follows:
    ///1. Figures out what inode the cwd corresponds to (skip this step if `path` is absolute)
    ///2. Reads `path` through the file system
    ///
    ///Returns an error if
    /// - the path is invalid
    /// - any directory, referenced in the path (or the cwd), does not exist
    /// - any of the intermediate names refers to a file that is not of directory type.
    fn resolve_path(&self, path: &str) -> Result<Self::Inode, Self::Error>;

    ///Create a new directory at the given path, where the last name of the path is the name for the new directory.
    ///For example, the path `/test/dir` will create a directory named `dir` in the parent directory `test`.
    ///Returns the newly created inode in case of success. Note that the newly created directory is already referenced once, i.e. its `nlink` field is not 0.
    ///
    ///New directories are created with 2 default entries in them, i.e. "." and ".." (in this order), that point to the current directory's inode and its parent's inode, respectively. This allows us to easily go back to the parent directory of the current directory, and to succinctly reference sibling files and folders in the same directory.
    ///Note that the addition of ".." causes the `nlink` field of the parent directory to increase by 1.
    ///
    ///Error
    /// - if the path is not valid
    /// - if the path's prefix (i.e. the part of the path without the name of the directory that we are about to create) does not exist in the file system yet.
    /// - if the last part of the path is not a valid directory name (i.e. it cannot be "." or "..")
    fn mkdir(&mut self, path: &str) -> Result<Self::Inode, Self::Error>;

    ///Remove the directory entry located at path `path`, i.e. set the `inum` of this entry to 0 and the name of the entry to all zeroes, i.e. to `"0".repeat(*DIRNAME_SIZE)`
    ///For example, the path `/test/dir` will (on success) delete the directory entry "dir" in the parent directory `test`.
    ///In essence, the inverse of `mkdir`.
    ///
    ///Decreases the `nlink` field of the deleted entry by 1 (unless we just deleted a cyclic reference to the parent), and frees the inode in case `nlink` decreases to 0. In the latter case, a reference to the inode also gets deleted (since ".." in the entry's inode disappears when it is freed)
    ///
    ///Errors and does nothing else in the following cases:
    ///- the path ends in "." or ".."; these entries cannot be removed
    ///- the path is not valid
    ///- the path does not exist yet in the file system, which includes the next bullet;
    ///- the entry is not present in the directory
    ///- the entry we are about to delete is itself a directory and non-empty (apart from the 2 default entries) - note: you cannot judge emptiness just from the size of the file, as it might contain directory entries that were previously unlinked as well
    fn unlink(&mut self, path: &str) -> Result<(), Self::Error>;
}

/// Support caching for inodes. Read more about what exactly this entails in assignment [`g_caching_inodes.rs`](../../cplwm_sol/g_caching_inodes/index.html) in the solution folder.
pub trait InodeCacheSupport: InodeSupport {
    /// `i_get_mut` is a new version of `i_get` that takes a mutable reference to self.
    ///This is required since getting an inode from the disk potentially alters the cache.
    ///The original implementation of `i_get` should now only read values from the cache, and not fetch them from disk, so that `self` does not have to be mutable there
    ///This function then subsumes the behavior of `iget`, by behaving in the following way;
    ///
    ///Finds the inode corresponding to `i` and returns it.
    ///Looks in the cache first and returns a copy if the inode is present.
    ///If the inode is not there, reads it from disk and puts it in the cache.
    ///When looking for an entry to replace in the cache, `i_get_mut` will evict the first entry it finds that currently has no other references to it (you can check this using the `strong_count` method of the `Rc` type). *Before evicting the old entry, make sure to persist its contents to the disk.*
    ///After eviction, the read `DInode` is wrapped in a `CachedInode`, written into the cache, and a copy of it is returned to the caller.
    fn i_get_mut(&mut self, i: u64) -> Result<Self::Inode, Self::Error>;

    ///Is the given inode currently in the inode cache?
    fn is_cached(&self, inum: u64) -> bool;

    ///Alternative version of `mkfs`, that allows us to specify the number of entries in the inode cache.
    ///Interpret the original `mfks` function as a more specific variant of this function, where the number of cache entries for inodes is fixed to 5.
    fn mkfs_cached<P: AsRef<Path>>(
        path: P,
        sb: &SuperBlock,
        nb_cache_entries: u64,
    ) -> Result<Self, Self::Error>;

    ///Alternative version of `mountfs`, that allows us to specify the number of entries in the inode cache.
    ///Interpret the original `mountfs` function as a more specific variant of this function, where the number of cache entries for inodes is fixed to 5.
    fn mountfs_cached(dev: Device, nb_cache_entries: u64) -> Result<Self, Self::Error>;
}
