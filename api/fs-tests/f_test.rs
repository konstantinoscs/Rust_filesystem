use super::FSName;
use cplfs_api::fs::{BlockSupport, FileSysSupport, InodeRWSupport, InodeSupport};
use cplfs_api::types::{Buffer, FType, InodeLike, SuperBlock, DIRECT_POINTERS};
use std::path::PathBuf;

#[path = "utils.rs"]
mod utils;

static BLOCK_SIZE: u64 = 300;
static NBLOCKS: u64 = 40;
static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE,
    nblocks: NBLOCKS,
    ninodes: 6,
    inodestart: 1,
    ndatablocks: 30,
    bmapstart: 4,
    datastart: 5,
};

fn disk_prep_path(name: &str) -> PathBuf {
    utils::disk_prep_path(&("fs-images-f-".to_string() + name), "img")
}

//Check creation of inode with indirect block
#[test]
fn basics() {
    let i1 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        1,
        &FType::TFile,
        0,
        2 * BLOCK_SIZE,
        &(0..(DIRECT_POINTERS + 1)).collect::<Vec<_>>()[..],
    )
    .unwrap();
    assert_eq!(i1.get_block(DIRECT_POINTERS), DIRECT_POINTERS);
    assert_eq!(i1.get_block(DIRECT_POINTERS + 1), 0);
}

#[test]
fn read_write() {
    let path = disk_prep_path("read_write");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //empty inode
    let mut i1 =
        <<FSName as InodeSupport>::Inode as InodeLike>::new(1, &FType::TFile, 0, 0, &[]).unwrap();

    my_fs.i_put(&i1).unwrap();

    //Some data
    let buf5000 = Buffer::new(vec![6; 5000].into_boxed_slice());

    //Try to perform some operations:
    my_fs.i_write(&mut i1, &buf5000, 0, 5000).unwrap();
    let mut i1 = my_fs.i_get(1).unwrap();
    assert_eq!(i1.get_size(), 5000);
    assert_ne!(i1.get_block(DIRECT_POINTERS), 0); //should have allocated an indirect block, and not zero

    let mut buf5000r = Buffer::new(vec![0; 5000].into_boxed_slice());
    assert_eq!(my_fs.i_read(&mut i1, &mut buf5000r, 0, 5000).unwrap(), 5000);

    assert_eq!(buf5000, buf5000r);

    //shouldve allocated ceil(5000/300) +  1 -> 18 blocks
    for i in 0..18 {
        my_fs.b_free(i).unwrap();
    }
    assert!(my_fs.i_free(18).is_err()); //not allocated

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn free() {
    let path = disk_prep_path("free");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    //empty inode
    let mut i1 =
        <<FSName as InodeSupport>::Inode as InodeLike>::new(1, &FType::TFile, 0, 0, &[]).unwrap();

    my_fs.i_put(&i1).unwrap();

    //Some data
    let buf5000 = Buffer::new(vec![6; 5000].into_boxed_slice());

    //Try to perform some operations:
    my_fs.i_write(&mut i1, &buf5000, 0, 5000).unwrap();
    let i1 = my_fs.i_get(1).unwrap();
    assert_eq!(i1.get_size(), 5000);
    assert_ne!(i1.get_block(DIRECT_POINTERS), 0); //should have allocated an indirect block, and not zero

    my_fs.i_free(1).unwrap();
    //shouldve deallocated all
    for i in 0..30 {
        assert!(my_fs.i_free(i).is_err()); //not allocated
    }

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}
