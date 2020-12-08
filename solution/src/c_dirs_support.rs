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
//! COMPLETED: ?
//!
//! COMMENTS:
//!
//! ...
//!

use cplfs_api::controller::Device;
use cplfs_api::fs::{BlockSupport, FileSysSupport, InodeSupport, DirectorySupport};
use cplfs_api::types::{Block, FType, Inode, SuperBlock, DirEntry, DIRNAME_SIZE, DInode, InodeLike};
use std::path::Path;

use super::error_fs::DirLayerError;
use crate::b_inode_support::InodeLayerFS;
use std::cmp::min;

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
    fn sup_as_ref(&self) -> &SuperBlock {
        return self.inode_fs.sup_as_ref();
    }
}


impl FileSysSupport for DirLayerFS {
    type Error = DirLayerError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        InodeLayerFS::sb_valid(sb)
    }

    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error> {
        let mut inode_fs = InodeLayerFS::mkfs(path, sb)?;
        let root = <<Self as InodeSupport>::Inode as InodeLike>::new(
            1,
            &FType::TDir,
            1,
            0,
            &[]
        ).ok_or(DirLayerError::DirLayerOp("Couldn't initialize the filesystem"))?;
        inode_fs.i_put(&root);
        Ok(DirLayerFS {
            inode_fs
        })
    }

    fn mountfs(dev: Device) -> Result<Self, Self::Error> {
        Ok(DirLayerFS {
            inode_fs: InodeLayerFS::mountfs(dev)?
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
    }}

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
            return Option::None
        }
        let mut dir_entry = DirEntry {
            inum,
            name: Default::default()
        };
        Self::set_name_str(&mut dir_entry, name);
        Option::Some(dir_entry)
    }

    fn get_name_str(de: &DirEntry) -> String {
        let mut name :String = "".to_string();
        for ch in de.name.iter() {
            if *ch == '\0' {
                break;
            }
            name.push(*ch);
        }
        name
    }

    fn set_name_str(de: &mut DirEntry, name: &str) -> Option<()> {
        if name.len() == 0 || name.len() > DIRNAME_SIZE || !name.chars().all(char::is_alphanumeric) {
            return Option::None;
        }
        for (i,c) in name.chars().enumerate() {
            de.name[i] = c;
        }
        if name.len() < DIRNAME_SIZE -1 {
            de.name[name.len()+1] = '\0';
        }
        Option::Some(())
    }

    fn dirlookup(&self, inode: &Self::Inode, name: &str) -> Result<(Self::Inode, u64), Self::Error> {
        if inode.get_ft() != FType::TDir {
            return Err(DirLayerError::DirLayerInput("The given inode does not represent a Directory"));
        }
        unimplemented!()
    }

    fn dirlink(&mut self, inode: &mut Self::Inode, name: &str, inum: u64) -> Result<u64, Self::Error> {
        unimplemented!()
    }
}

// **TODO** define your own tests here.

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "c", feature = "all")))]
#[path = "../../api/fs-tests/c_test.rs"]
mod tests;
