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

use crate::c_dirs_support::DirLayerFS;
use crate::error_fs::PathError;
use cplfs_api::controller::Device;
use cplfs_api::fs::{BlockSupport, DirectorySupport, FileSysSupport, InodeSupport, PathSupport};
use cplfs_api::types::{Block, DirEntry, FType, Inode, InodeLike, SuperBlock, ROOT_INUM};
use relative_path::RelativePath;
use std::path::Path;

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
pub type FSName = PathFS;

///Struct representing a file system with up to Directory layer support
#[derive(Debug)]
pub struct PathFS {
    dir_fs: DirLayerFS,
    cur_dir: String,
}

impl PathFS {
    /// function to get full path of a given path with respect to the root
    /// if the filesystem didn't have hacky links, it would work also for resolving
    fn get_full_path(&self, path: &str) -> String {
        let mut full_path: String;
        if Path::new(path).has_root() {
            full_path = RelativePath::new(&path).normalize().to_string();
        } else {
            full_path = RelativePath::new(&self.cur_dir)
                .join_normalized(path)
                .to_string();
        }

        //if the we end up with a new root that goes back from root, then we should ignore the "../"s
        if !full_path.starts_with("/") {
            let mut names: Vec<&str> = full_path.split("/").collect();
            names.retain(|x| *x != "..");
            full_path = names.join("/");
            if !full_path.starts_with("/") {
                full_path = "/".to_string() + &full_path
            }
        }
        full_path
    }
}

impl FileSysSupport for PathFS {
    type Error = PathError;

    fn sb_valid(sb: &SuperBlock) -> bool {
        DirLayerFS::sb_valid(sb)
    }

    fn mkfs<P: AsRef<Path>>(path: P, sb: &SuperBlock) -> Result<Self, Self::Error> {
        let mut dir_fs = DirLayerFS::mkfs(path, sb)?;
        let mut root = dir_fs.i_get(1)?;
        dir_fs.dirlink(&mut root, ".", 1)?;
        dir_fs.dirlink(&mut root, "..", 1)?;

        Ok(PathFS {
            dir_fs,
            cur_dir: String::from("/"),
        })
    }

    fn mountfs(dev: Device) -> Result<Self, Self::Error> {
        Ok(PathFS {
            dir_fs: DirLayerFS::mountfs(dev)?,
            cur_dir: String::from("/"),
        })
    }

    fn unmountfs(self) -> Device {
        self.dir_fs.unmountfs()
    }
}

impl BlockSupport for PathFS {
    fn b_get(&self, i: u64) -> Result<Block, Self::Error> {
        Ok(self.dir_fs.b_get(i)?)
    }

    fn b_put(&mut self, b: &Block) -> Result<(), Self::Error> {
        Ok(self.dir_fs.b_put(b)?)
    }

    fn b_free(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.dir_fs.b_free(i)?)
    }

    fn b_zero(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.dir_fs.b_zero(i)?)
    }

    fn b_alloc(&mut self) -> Result<u64, Self::Error> {
        Ok(self.dir_fs.b_alloc()?)
    }

    fn sup_get(&self) -> Result<SuperBlock, Self::Error> {
        Ok(self.dir_fs.sup_get()?)
    }

    fn sup_put(&mut self, sup: &SuperBlock) -> Result<(), Self::Error> {
        Ok(self.dir_fs.sup_put(sup)?)
    }
}

impl InodeSupport for PathFS {
    type Inode = Inode;

    fn i_get(&self, i: u64) -> Result<Self::Inode, Self::Error> {
        Ok(self.dir_fs.i_get(i)?)
    }

    fn i_put(&mut self, ino: &Self::Inode) -> Result<(), Self::Error> {
        Ok(self.dir_fs.i_put(ino)?)
    }

    fn i_free(&mut self, i: u64) -> Result<(), Self::Error> {
        Ok(self.dir_fs.i_free(i)?)
    }

    fn i_alloc(&mut self, ft: FType) -> Result<u64, Self::Error> {
        Ok(self.dir_fs.i_alloc(ft)?)
    }

    fn i_trunc(&mut self, inode: &mut Self::Inode) -> Result<(), Self::Error> {
        Ok(self.dir_fs.i_trunc(inode)?)
    }
}

impl DirectorySupport for PathFS {
    fn new_de(inum: u64, name: &str) -> Option<DirEntry> {
        DirLayerFS::new_de(inum, name)
    }

    fn get_name_str(de: &DirEntry) -> String {
        DirLayerFS::get_name_str(de)
    }

    fn set_name_str(de: &mut DirEntry, name: &str) -> Option<()> {
        DirLayerFS::set_name_str(de, name)
    }

    fn dirlookup(
        &self,
        inode: &Self::Inode,
        name: &str,
    ) -> Result<(Self::Inode, u64), Self::Error> {
        Ok(self.dir_fs.dirlookup(inode, name)?)
    }

    fn dirlink(
        &mut self,
        inode: &mut Self::Inode,
        name: &str,
        inum: u64,
    ) -> Result<u64, Self::Error> {
        Ok(self.dir_fs.dirlink(inode, name, inum)?)
    }
}

impl PathSupport for PathFS {
    fn valid_path(path: &str) -> bool {
        if path == "/" {
            return true;
        }
        if path.is_empty() {
            return false;
        }
        if !path.starts_with("../") && !path.starts_with("./") && !path.starts_with("/") {
            return false;
        }
        if path.ends_with("/") {
            return false;
        }
        let mut names: Vec<&str> = path.split("/").collect();
        if names[0] == "" {
            names.remove(0);
        }
        for name in names {
            if !DirLayerFS::is_valid_dir_name(name) {
                return false;
            }
        }
        true
    }

    fn get_cwd(&self) -> String {
        self.cur_dir.clone()
    }

    fn set_cwd(&mut self, path: &str) -> Option<()> {
        if !Self::valid_path(path) {
            return Option::None;
        }

        self.cur_dir = String::from(self.get_full_path(path));
        println!("Set cwd to {}", self.get_cwd());
        Some(())
    }

    fn resolve_path(&self, path: &str) -> Result<Self::Inode, Self::Error> {
        if !Self::valid_path(path) {
            return Err(PathError::InvalidPathName(path.to_string()));
        }
        //formulate the correct full path to look for
        let full_path: String;
        if Path::new(path).has_root() {
            full_path = path.to_string();
        } else if self.get_cwd() == "/" {
            full_path = "/".to_string() + path;
        } else {
            full_path = self.get_cwd() + "/" + path;
        }

        let mut cur_inode = self.i_get(ROOT_INUM)?;
        for dir in full_path.split("/").skip(1) {
            if cur_inode.get_ft() != FType::TDir {
                return Err(PathError::InodeNotDir(dir.to_string()));
            }
            cur_inode = self.dirlookup(&cur_inode, dir)?.0;
        }
        Ok(cur_inode)
    }

    fn mkdir(&mut self, _path: &str) -> Result<Self::Inode, Self::Error> {
        unimplemented!()
    }

    fn unlink(&mut self, _path: &str) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "d", feature = "all")))]
#[path = "../../api/fs-tests/d_test.rs"]
mod tests;
