use super::FSName;
use cplfs_api::fs::{BlockSupport, FileSysSupport, InodeSupport};
use cplfs_api::types::{FType, InodeLike, SuperBlock};
use std::path::PathBuf;

#[path = "utils.rs"]
mod utils;

static BLOCK_SIZE: u64 = 1000;
static NBLOCKS: u64 = 10;
static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE,
    nblocks: NBLOCKS,
    ninodes: 6,
    inodestart: 1,
    ndatablocks: 5,
    bmapstart: 4,
    datastart: 5,
};

fn disk_prep_path(name: &str) -> PathBuf {
    utils::disk_prep_path(&("fs-images-b-".to_string() + name), "img")
}

//Note that none of the below tests will test the situation where the inodes span multiple blocks. You should test this yourself.
#[test]
fn mkfs() {
    let path = disk_prep_path("mkfs");

    //A working one
    let my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();
    let sb = my_fs.b_get(0).unwrap();
    assert_eq!(
        sb.deserialize_from::<SuperBlock>(0).unwrap(),
        SUPERBLOCK_GOOD
    );
    assert_eq!(my_fs.sup_get().unwrap(), SUPERBLOCK_GOOD);

    //Checks on proper init of inodes
    assert_eq!(my_fs.i_get(1).unwrap().get_ft(), FType::TFree); //Don't sneak your directory support in here yet
    assert_eq!(my_fs.i_get(5).unwrap().get_ft(), FType::TFree);
    assert!(my_fs.i_get(6).is_err());

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn get_put() {
    let path = disk_prep_path("get_put");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let i1 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        1,
        &FType::TFile,
        0,
        2 * BLOCK_SIZE,
        &[2, 3],
    )
    .unwrap();
    let i2 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        5,
        &FType::TFree,
        1,
        (2.5 * (BLOCK_SIZE as f32)) as u64,
        &[2, 3, 4],
    )
    .unwrap();
    let b1 = my_fs.b_get(SUPERBLOCK_GOOD.inodestart).unwrap();

    my_fs.i_put(&i1).unwrap();
    my_fs.i_put(&i2).unwrap();

    assert_eq!(my_fs.i_get(1).unwrap(), i1);
    assert_eq!(my_fs.i_get(5).unwrap(), i2);

    let dev = my_fs.unmountfs();
    assert_ne!(b1, dev.read_block(SUPERBLOCK_GOOD.inodestart).unwrap()); //Did you actually persist
    utils::disk_destruct(dev);
}

#[test]
fn free_alloc() {
    let path = disk_prep_path("free_alloc");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //Allocate
    for i in 0..(SUPERBLOCK_GOOD.ninodes - 1) {
        assert_eq!(my_fs.i_alloc(FType::TFile).unwrap(), i + 1); //Note; allocations starts from 1
    }
    let i1 = my_fs.i_get(2).unwrap();
    assert_eq!(i1.get_ft(), FType::TFile);
    assert_eq!(i1.get_size(), 0);
    assert_eq!(i1.get_nlink(), 0);

    assert!(my_fs.i_alloc(FType::TDir).is_err()); //No more blocks

    //Dealloc 2, realloc 1
    my_fs.i_free(5).unwrap();
    my_fs.i_free(2).unwrap();
    assert_eq!(my_fs.i_get(2).unwrap().get_ft(), FType::TFree);
    assert_eq!(my_fs.i_alloc(FType::TDir).unwrap(), 2);
    assert_eq!(my_fs.i_get(2).unwrap().get_ft(), FType::TDir);

    //Do nothing if inode has nlink neq to zero
    let i1 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        2,
        &FType::TFile,
        1,
        (2.5 * (BLOCK_SIZE as f32)) as u64,
        &[2, 3, 4],
    )
    .unwrap();
    my_fs.i_put(&i1).unwrap();
    assert!(my_fs.b_free(2).is_err());
    assert_eq!(my_fs.i_get(2).unwrap().get_ft(), FType::TFile);

    //Allocate blocks 5-6-7-8
    for i in 0..4 {
        assert_eq!(my_fs.b_alloc().unwrap(), i);
    }
    let i2 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        3,
        &FType::TFile,
        0,
        (1.5 * (BLOCK_SIZE as f32)) as u64,
        &[7, 8],
    )
    .unwrap();
    my_fs.i_put(&i2).unwrap();
    my_fs.i_free(3).unwrap();
    //Already freed
    assert!(my_fs.b_free(2).is_err()); //watch out; absolute indices
    assert!(my_fs.b_free(3).is_err());

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn itrunc() {
    let path = disk_prep_path("itrunc");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //Allocate blocks 5-6-7-8
    for i in 0..5 {
        assert_eq!(my_fs.b_alloc().unwrap(), i);
    }
    let i2 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        2,
        &FType::TFile,
        0,
        (1.5 * (BLOCK_SIZE as f32)) as u64,
        &[6, 7, 8],
    )
    .unwrap();
    my_fs.i_put(&i2).unwrap();
    let mut i3 = my_fs.i_get(2).unwrap();
    my_fs.i_trunc(&mut i3).unwrap();
    assert_eq!(my_fs.i_get(2).unwrap(), i3);
    assert_eq!(i3.get_ft(), FType::TFile);
    assert_eq!(i3.get_size(), 0);
    assert_eq!(i3.get_nlink(), 0);

    //Already freed
    assert!(my_fs.b_free(1).is_err());
    assert!(my_fs.b_free(2).is_err());
    assert!(my_fs.b_free(3).is_ok()); //sneaky; not deallocated

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}
