//! File system with directory support
//!
//! Create a filesystem that has a notion of blocks, inodes and directory inodes, by implementing the [`FileSysSupport`], the [`BlockSupport`], the [`InodeSupport`] and the [`DirectorySupport`] traits together (again, all earlier traits are supertraits of the later ones).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
//! [`DirectorySupport`]: ../../cplfs_api/fs/trait.DirectorySupport.html
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

use cplfs_api::controller::Device;
use cplfs_api::fs::{BlockSupport, DirectorySupport, FileSysSupport, InodeRWSupport, InodeSupport};
use cplfs_api::types::{
    Block, Buffer, DInode, DirEntry, FType, Inode, InodeLike, SuperBlock, DIRENTRY_SIZE,
    DIRNAME_SIZE,
};
use std::path::Path;

use super::error_fs::DirLayerError;
use crate::b_inode_support::InodeLayerFS;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
/// **TODO**: replace the below type by the type of your file system
pub type FSName = DirLayerFS;

///Struct representing a file system with up to Directory layer support
#[derive(Debug)]
pub struct DirLayerFS {
    inode_fs: InodeLayerFS,
}

impl DirLayerFS {
    fn eq_str_char_arr(&self, string: &str, arr: &[char]) -> bool {
        let arrlen = arr.iter().filter(|&c| *c != '\0').count();
        if string.len() != arrlen {
            return false;
        }
        for (i, c) in string.chars().enumerate() {
            if arr[i] != c {
                return false;
            }
        }
        true
    }

    fn get_dir_entry(
        &self,
        inode: &<Self as InodeSupport>::Inode,
        idx: u64,
    ) -> Result<DirEntry, <Self as FileSysSupport>::Error> {
        let mut buf = Buffer::new_zero(*DIRENTRY_SIZE);
        self.inode_fs
            .i_read(inode, &mut buf, idx * (*DIRENTRY_SIZE), *DIRENTRY_SIZE)?;
        Ok(buf.deserialize_from::<DirEntry>(0)?)
    }

    /// checks if a string represents a valid directory name
    pub fn is_valid_dir_name(name: &str) -> bool {
        //println!("Checking {}", name);
        name == ".."
            || name == "."
            || (name.len() != 0
                && name.len() <= DIRNAME_SIZE
                && name.chars().all(char::is_alphanumeric))
    }
}

impl FileSysSupport for DirLayerFS {
    type Error = DirLayerError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        InodeLayerFS::sb_valid(sb)
    }

    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error> {
        let mut inode_fs = InodeLayerFS::mkfs(path, sb)?;
        let root = <<Self as InodeSupport>::Inode as InodeLike>::new(1, &FType::TDir, 1, 0, &[])
            .ok_or(DirLayerError::DirLayerOp(
                "Couldn't initialize the filesystem",
            ))?;
        inode_fs.i_put(&root)?;
        Ok(DirLayerFS { inode_fs })
    }

    fn mountfs(dev: Device) -> Result<Self, Self::Error> {
        Ok(DirLayerFS {
            inode_fs: InodeLayerFS::mountfs(dev)?,
        })
    }

    fn unmountfs(self) -> Device {
        self.inode_fs.unmountfs()
    }
}

impl BlockSupport for DirLayerFS {
    fn b_get(&self, i: u64) -> Result<Block, Self::Error> {
        Ok(self.inode_fs.b_get(i)?)
    }

    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error> {
        Ok(self.inode_fs.b_put(b)?)
    }

    fn b_free(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.inode_fs.b_free(i)?)
    }

    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.inode_fs.b_zero(i)?)
    }

    fn b_alloc(&mut self) -> Result<u64, Self::Error> {
        Ok(self.inode_fs.b_alloc()?)
    }

    fn sup_get(&self) -> Result<SuperBlock, Self::Error> {
        Ok(self.inode_fs.sup_get()?)
    }

    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error> {
        Ok(self.inode_fs.sup_put(sup)?)
    }
}

impl InodeSupport for DirLayerFS {
    type Inode = Inode;

    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error> {
        Ok(self.inode_fs.i_get(i)?)
    }

    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {
        Ok(self.inode_fs.i_put(ino)?)
    }

    fn i_free(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.inode_fs.i_free(i)?)
    }

    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error> {
        Ok(self.inode_fs.i_alloc(ft)?)
    }

    fn i_trunc(&mut self, inode: &mut Self::Inode) -> Result<(), Self::Error> {
        Ok(self.inode_fs.i_trunc(inode)?)
    }
}

impl DirectorySupport for DirLayerFS {
    fn new_de(inum: u64, name: &str) -> Option<DirEntry> {
        if name.len() == 0 {
            return Option::None;
        }
        let mut dir_entry = DirEntry {
            inum,
            name: Default::default(),
        };
        match Self::set_name_str(&mut dir_entry, name) {
            None => Option::None,
            Some(_) => Option::Some(dir_entry),
        }
    }

    fn get_name_str(de: &DirEntry) -> String {
        let mut name: String = "".to_string();
        for ch in de.name.iter() {
            if *ch == '\0' {
                break;
            }
            name.push(*ch);
        }
        name
    }

    fn set_name_str(de: &mut DirEntry, name: &str) -> Option<()> {
        if !Self::is_valid_dir_name(name) {
            return Option::None;
        }
        for (i, c) in name.chars().enumerate() {
            de.name[i] = c;
        }
        if name.len() < DIRNAME_SIZE - 1 {
            de.name[name.len() + 1] = '\0';
        }
        Option::Some(())
    }

    fn dirlookup(
        &self,
        inode: &Self::Inode,
        name: &str,
    ) -> Result<(Self::Inode, u64), Self::Error> {
        if inode.get_ft() != FType::TDir {
            return Err(DirLayerError::DirLayerInput(
                "The given inode does not represent a Directory",
            ));
        }
        // start grabbing DirEntries and seeing if they are the one we are looking for
        let no_entries = inode.get_size() / (*DIRENTRY_SIZE);
        for i in 0..no_entries {
            let entry = self.get_dir_entry(inode, i)?;
            if self.eq_str_char_arr(name, &entry.name) {
                return Ok((self.i_get(entry.inum)?, i * (*DIRENTRY_SIZE)));
            }
        }
        Err(DirLayerError::DirLookupNotFound())
    }

    fn dirlink(
        &mut self,
        inode: &mut Self::Inode,
        name: &str,
        inum: u64,
    ) -> Result<u64, Self::Error> {
        // First check if inode is a dir, doesn't contain an entry with 'name'
        // and the inode with 'inum' is already allocated
        if inode.get_ft() != FType::TDir {
            return Err(DirLayerError::DirLayerInput(
                "The given inode does not correspond to a directory",
            ));
        }
        match self.dirlookup(inode, name) {
            Err(DirLayerError::DirLookupNotFound()) => {}
            Ok(_) => {
                return Err(DirLayerError::DirLayerInput(
                    "The given node contains a dir entry with the same name",
                ))
            }
            Err(e) => return Err(e),
        }
        let mut queried_inode = self.i_get(inum)?;
        if queried_inode.get_ft() == FType::TFree {
            return Err(DirLayerError::DirLayerInput(
                "The given inum points to a free inode",
            ));
        }

        let entry = Self::new_de(inum, name).ok_or(DirLayerError::DirLayerOp(
            "Could not initialize new dirEntry",
        ))?;
        let mut t_offest = inode.get_size();

        // try to see if there is some free DirEntry
        let no_entries = inode.get_size() / (*DIRENTRY_SIZE);
        for i in 0..no_entries {
            if self.get_dir_entry(inode, i)?.inum == 0 {
                t_offest = i * (*DIRENTRY_SIZE);
                break;
            }
        }

        let mut buf = Buffer::new_zero(*DIRENTRY_SIZE);
        buf.serialize_into(&entry, 0)?;
        self.inode_fs
            .i_write(inode, &buf, t_offest, *DIRENTRY_SIZE)?;
        if inum != inode.get_inum() {
            queried_inode.disk_node.nlink += 1;
            self.i_put(&queried_inode)?;
        }
        Ok(t_offest)
    }
}

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "c", feature = "all")))]
#[path = "../../api/fs-tests/c_test.rs"]
mod tests;
