use super::FSName;
use cplfs_api::fs::{BlockSupport, DirectorySupport, FileSysSupport, InodeSupport};
use cplfs_api::types::{FType, InodeLike, SuperBlock, DIRENTRY_SIZE};
use std::path::PathBuf;

#[path = "utils.rs"]
mod utils;

static BLOCK_SIZE: u64 = 1000;
static NBLOCKS: u64 = 10;
static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE,
    nblocks: NBLOCKS,
    ninodes: 8,
    inodestart: 1,
    ndatablocks: 5,
    bmapstart: 4,
    datastart: 5,
};

fn disk_prep_path(name: &str) -> PathBuf {
    utils::disk_prep_path(&("fs-images-c-".to_string() + name), "img")
}

#[test]
fn de_utils() {
    let path = disk_prep_path("mkfs");
    let my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let name1 = "test"; //should stop reading at the end string char
    let mut de = FSName::new_de(0, name1).unwrap();
    assert_eq!("test", FSName::get_name_str(&de));
    let name2 = "verylongname";
    FSName::set_name_str(&mut de, name2).unwrap();
    assert_eq!(name2, FSName::get_name_str(&de));
    let name = "nowthisoneisreallylong";
    assert!(FSName::set_name_str(&mut de, name).is_none());
    let name = "";
    assert!(FSName::set_name_str(&mut de, name).is_none());
    let name = "❤";
    assert!(FSName::set_name_str(&mut de, name).is_none());

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn root() {
    let path = disk_prep_path("root");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();
    assert_eq!(my_fs.i_get(1).unwrap().get_ft(), FType::TDir);
    assert_eq!(my_fs.i_get(0).unwrap().get_ft(), FType::TFree);
    my_fs.i_free(1).unwrap(); //inode has been allocated, but should not be deallocated, as the root references itself
    my_fs.i_free(1).unwrap(); //so this should work twice
    assert!(my_fs.i_free(0).is_err()); //inode has not been allocated
    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

//does not test multi block allocations
#[test]
fn dirlookup_link() {
    let path = disk_prep_path("lkup_link");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let mut i1 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        1,
        &FType::TFile,
        0,
        2 * BLOCK_SIZE,
        &[2, 3],
    )
    .unwrap();
    assert!(my_fs.dirlink(&mut i1, "test", 10).is_err());
    assert!(my_fs.dirlookup(&i1, "test").is_err());

    let mut i2 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        5,
        &FType::TDir,
        0,
        (2.5 * (BLOCK_SIZE as f32)) as u64,
        &[6, 7, 8], //All of these blocks are initially 0'ed -> free direntries
    )
    .unwrap();
    my_fs.i_put(&i2).unwrap();

    //Allocate blocks 5-6-7
    for i in 0..3 {
        assert_eq!(my_fs.b_alloc().unwrap(), i);
    }

    //Allocate inodes 2,3,4
    for i in 0..3 {
        assert_eq!(my_fs.i_alloc(FType::TFile).unwrap(), i + 2);
    }

    assert!(my_fs.dirlink(&mut i2, "test1", 6).is_err()); //uncallocated
    assert_eq!(my_fs.dirlink(&mut i2, "test1", 2).unwrap(), 0);
    assert_eq!(my_fs.dirlink(&mut i2, "test2", 3).unwrap(), *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&i2, "test2").unwrap().1, *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&i2, "test2").unwrap().0.inum, 3);
    assert_eq!(
        my_fs.dirlookup(&mut i2, "test2").unwrap().0.disk_node.nlink,
        1
    );
    for i in 0..5 {
        assert_eq!(
            my_fs.dirlink(&mut i2, &i.to_string(), 3).unwrap(),
            (i + 2) * *DIRENTRY_SIZE
        );
    }
    assert_eq!(my_fs.dirlookup(&i2, "3").unwrap().0.disk_node.nlink, 6);

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn dirlookup_link_root() {
    let path = disk_prep_path("lkup_link_root");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //Get the root node
    let mut iroot = my_fs.i_get(1).unwrap();

    //Unallocated
    assert!(my_fs.dirlink(&mut iroot, &0.to_string(), 3).is_err());
    //Allocate inodes 2,3,4
    for i in 0..3 {
        assert_eq!(my_fs.i_alloc(FType::TFile).unwrap(), i + 2);
    }
    for i in 0..5 {
        assert_eq!(
            my_fs.dirlink(&mut iroot, &i.to_string(), 3).unwrap(),
            i * *DIRENTRY_SIZE
        );
    }
    assert_eq!(my_fs.dirlookup(&iroot, "4").unwrap().1, 4 * *DIRENTRY_SIZE);
    assert_eq!(iroot.get_size(), 5 * *DIRENTRY_SIZE);

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}
