use super::FSName;
use cplfs_api::fs::{BlockSupport, FileSysSupport, InodeRWSupport, InodeSupport};
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

fn disk_prep_path(name: &str) -> PathBuf {
    utils::disk_prep_path(&("fs-images-e-".to_string() + name), "img")
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
    let mut buf50 = Buffer::new(vec![5; 50].into_boxed_slice());
    let mut buf500 = Buffer::new(vec![6; 500].into_boxed_slice());

    //Try to perform some operations:

    //One regular one that only changes the size
    let mut write_result = vec![4; 103];
    write_result.append(&mut vec![5; 50]);
    write_result.append(&mut vec![4; 147]);
    assert!(my_fs.i_write(&mut i2, &mut buf50, 703, 50).is_ok());
    assert_eq!(my_fs.i_get(2).unwrap(), i2);
    assert_eq!(753, i2.get_size());
    assert_eq!(my_fs.b_get(7).unwrap().contents_as_ref(), &write_result[..]);

    //And one that will require mapping in new blocks
    let mut write_result_2 = vec![6; 52];
    write_result_2.append(&mut vec![0; 248]);
    my_fs.i_write(&mut i2, &mut buf500, 753, 499).unwrap();
    assert_eq!(my_fs.i_get(2).unwrap(), i2);
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
