use super::FSName;
use cplfs_api::fs::{
    BlockSupport, FileSysSupport, InodeCacheSupport, InodeRWSupport, InodeSupport,
};
use cplfs_api::types::{Buffer, FType, InodeLike, SuperBlock};
use std::path::PathBuf;

#[path = "utils.rs"]
mod utils;

static BLOCK_SIZE: u64 = 300; //make blocks somewhat smaller on this one, should still be sufficient for a reasonable inode
static NBLOCKS: u64 = 11;
static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE,
    nblocks: NBLOCKS,
    ninodes: 6,
    inodestart: 1,
    ndatablocks: 6,
    bmapstart: 4,
    datastart: 5,
};

static BLOCK_SIZE_C: u64 = 1000; //make blocks somewhat smaller on this one, should still be sufficient for a reasonable inode
static SUPERBLOCK_GOOD_C: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE_C,
    nblocks: NBLOCKS,
    ninodes: 10,
    inodestart: 1,
    ndatablocks: 6,
    bmapstart: 4,
    datastart: 5,
};

fn disk_prep_path(name: &str) -> PathBuf {
    utils::disk_prep_path(&("fs-images-g-".to_string() + name), "img")
}

#[test]
fn check_cached() {
    let path = disk_prep_path("check_cache");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD_C).unwrap(); //5 cached

    for i in 0..5 {
        assert!(!my_fs.is_cached(i + 1));
        assert_eq!(my_fs.i_alloc(FType::TDir).unwrap(), i + 1);
        assert!(my_fs.is_cached(i + 1));
    }
    //0 should be evicted; cache size 5
    assert_eq!(my_fs.i_alloc(FType::TDir).unwrap(), 6);
    assert!(!my_fs.is_cached(1));
    my_fs.i_free(1).unwrap(); //should work
    assert!(my_fs.is_cached(6)); //should not have been evicted by the free
    assert!(my_fs.i_free(1).is_err()); //no double free

    assert_eq!(my_fs.i_alloc(FType::TDir).unwrap(), 1);
    assert!(my_fs.is_cached(1));
    assert!(!my_fs.is_cached(6));

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn aliasing() {
    let path = disk_prep_path("aliasing");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD_C).unwrap(); //5 cached

    let i1 =
        <<FSName as InodeSupport>::Inode as InodeLike>::new(1, &FType::TFile, 0, 0, &[]).unwrap();
    my_fs.i_put(&i1).unwrap();
    assert!(!my_fs.is_cached(1));
    let mut i1 = my_fs.i_get_mut(1).unwrap();
    assert!(my_fs.is_cached(1));
    let i1_alias = my_fs.i_get_mut(1).unwrap();
    assert!(my_fs.is_cached(1));

    //Check support for aliasing
    let mut buf500 = Buffer::new(vec![6; 500].into_boxed_slice());
    assert!(my_fs.i_write(&mut i1, &mut buf500, 0, 500).is_ok());
    assert_eq!(my_fs.i_get_mut(1).unwrap(), i1);
    assert_eq!(i1_alias, i1); //alias?
    assert!(my_fs.i_free(1).is_err()); //shouldnt work; still refs
    drop(i1);
    drop(i1_alias);
    my_fs.i_free(1).unwrap(); //should work now
    assert!(my_fs.i_free(1).is_err());

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

//***********************
//The rest is just a repetition and slight adaptation of some old tests, that should now still work
//**********************
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

    assert_eq!(my_fs.i_get_mut(1).unwrap(), i1);
    assert_eq!(my_fs.i_get_mut(5).unwrap(), i2);

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
    let i1 = my_fs.i_get_mut(2).unwrap();
    assert_eq!(i1.get_ft(), FType::TFile);
    assert_eq!(i1.get_size(), 0);
    assert_eq!(i1.get_nlink(), 0);

    assert!(my_fs.i_alloc(FType::TDir).is_err()); //No more blocks

    //Dealloc 2, realloc 1
    my_fs.i_free(5).unwrap();
    assert_eq!(i1.get_ft(), FType::TFile);
    drop(i1);
    my_fs.i_free(2).unwrap();
    assert_eq!(my_fs.i_get_mut(2).unwrap().get_ft(), FType::TFree);
    assert_eq!(my_fs.i_alloc(FType::TDir).unwrap(), 2);
    assert_eq!(my_fs.i_get_mut(2).unwrap().get_ft(), FType::TDir);
    my_fs.i_free(2).unwrap();

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
    let mut i3 = my_fs.i_get_mut(2).unwrap();
    my_fs.i_trunc(&mut i3).unwrap();
    assert_eq!(my_fs.i_get_mut(2).unwrap(), i3);
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

#[test]
fn error_cases() {
    let path = disk_prep_path("error_cases");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //Set up an inode with 3 blocks first
    for i in 0..5 {
        assert_eq!(my_fs.b_alloc().unwrap(), i);
    }
    let b2 = utils::n_block(5, BLOCK_SIZE, 2);
    my_fs.b_put(&b2).unwrap();
    let b3 = utils::n_block(6, BLOCK_SIZE, 3);
    my_fs.b_put(&b3).unwrap();
    let b4 = utils::n_block(7, BLOCK_SIZE, 4);
    my_fs.b_put(&b4).unwrap();
    let mut i2 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        2,
        &FType::TFile,
        0,
        (2.5 * (BLOCK_SIZE as f32)) as u64, //size is 750
        &[5, 6, 7],
    )
    .unwrap();
    my_fs.i_put(&i2).unwrap(); //Store the inode to disk as well

    //Some buffers
    let mut buf50 = Buffer::new_zero(50);
    let mut buf500 = Buffer::new_zero(500);

    //Try to perform some operations
    assert!(my_fs.i_read(&i2, &mut buf500, 751, 0).is_err());
    assert!(my_fs.i_write(&mut i2, &mut buf500, 751, 0).is_err());
    assert!(my_fs.i_write(&mut i2, &mut buf500, 750, 0).is_ok());
    assert_eq!(my_fs.i_read(&i2, &mut buf500, 750, 1).unwrap(), 0);
    assert!(my_fs.i_read(&i2, &mut buf50, 751, 51).is_err());

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn readi() {
    let path = disk_prep_path("readi");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //Set up an inode with 3 blocks first
    for i in 0..5 {
        assert_eq!(my_fs.b_alloc().unwrap(), i);
    }
    let b2 = utils::n_block(5, BLOCK_SIZE, 2);
    my_fs.b_put(&b2).unwrap();
    let b3 = utils::n_block(6, BLOCK_SIZE, 3);
    my_fs.b_put(&b3).unwrap();
    let b4 = utils::n_block(7, BLOCK_SIZE, 4);
    my_fs.b_put(&b4).unwrap();
    let i2 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        2,
        &FType::TFile,
        0,
        3 * BLOCK_SIZE, //size is 900
        &[5, 6, 7],
    )
    .unwrap();
    my_fs.i_put(&i2).unwrap(); //Store the inode to disk as well

    //Some buffers
    let mut buf50 = Buffer::new_zero(50);
    let mut buf500 = Buffer::new_zero(500);

    //Try to perform some operations
    let mut read_result = vec![2; 33];
    read_result.append(&mut vec![3; 300]);
    read_result.append(&mut vec![4; 155]);
    assert_eq!(my_fs.i_read(&i2, &mut buf500, 267, 488).unwrap(), 488);
    assert_eq!(
        &buf500.contents_as_ref()[..488].len(),
        &read_result[..].len()
    );
    assert_eq!(&buf500.contents_as_ref()[..488], &read_result[..]);

    let mut read_result_2 = vec![2; 33];
    read_result_2.append(&mut vec![3; 17]);
    assert_eq!(my_fs.i_read(&i2, &mut buf50, 267, 50).unwrap(), 50);
    assert_eq!(&buf50.contents_as_ref()[..], &read_result_2[..]);

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn writei() {
    let path = disk_prep_path("writei");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //Set up an inode with 3 blocks first
    for i in 0..4 {
        assert_eq!(my_fs.b_alloc().unwrap(), i);
    }
    let b2 = utils::n_block(5, BLOCK_SIZE, 2);
    my_fs.b_put(&b2).unwrap();
    let b3 = utils::n_block(6, BLOCK_SIZE, 3);
    my_fs.b_put(&b3).unwrap();
    let b4 = utils::n_block(7, BLOCK_SIZE, 4);
    my_fs.b_put(&b4).unwrap();
    let i2 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        2,
        &FType::TFile,
        0,
        (2.5 * (BLOCK_SIZE as f32)) as u64, //size is 750
        &[5, 6, 7],
    )
    .unwrap();
    my_fs.i_put(&i2).unwrap(); //Store the inode to disk as well
    let mut i2 = my_fs.i_get_mut(2).unwrap(); //Make sure it ends up cached, so we do not use the unused copy `i2`

    //Some buffers
    let mut buf50 = Buffer::new(vec![5; 50].into_boxed_slice());
    let mut buf500 = Buffer::new(vec![6; 500].into_boxed_slice());

    //Try to perform some operations:

    //One regular one that only changes the size
    let mut write_result = vec![4; 103];
    write_result.append(&mut vec![5; 50]);
    write_result.append(&mut vec![4; 147]);
    assert!(my_fs.i_write(&mut i2, &mut buf50, 703, 50).is_ok());
    assert_eq!(my_fs.i_get_mut(2).unwrap(), i2);
    assert_eq!(753, i2.get_size());
    assert_eq!(my_fs.b_get(7).unwrap().contents_as_ref(), &write_result[..]);

    //And one that will require mapping in new blocks
    let mut write_result_2 = vec![6; 52];
    write_result_2.append(&mut vec![0; 248]);
    my_fs.i_write(&mut i2, &mut buf500, 753, 499).unwrap();
    assert_eq!(my_fs.i_get_mut(2).unwrap(), i2);
    assert_eq!(1252, i2.get_size());
    assert_eq!(9, i2.get_block(3));
    assert_eq!(10, i2.get_block(4));
    assert_eq!(
        my_fs.b_get(10).unwrap().contents_as_ref(),
        &write_result_2[..]
    );
    my_fs.b_free(4).unwrap();

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}
