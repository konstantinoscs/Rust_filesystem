use super::FSName;
use cplfs_api::controller::Device;
use cplfs_api::fs::{BlockSupport, FileSysSupport};
use cplfs_api::types::SuperBlock;
use std::path::{Path, PathBuf};

#[path = "utils.rs"]
mod utils;

static BLOCK_SIZE: u64 = 1000;
static NBLOCKS: u64 = 10;
static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE, //Note; assumes at least 2 inodes fit in one block. This should be the case for any reasonable inode implementation you might come up with
    nblocks: NBLOCKS,
    ninodes: 6,
    inodestart: 1,
    ndatablocks: 5,
    bmapstart: 4,
    datastart: 5,
};

static SUPERBLOCK_BAD_INODES: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE,
    nblocks: NBLOCKS,
    ninodes: 1000,
    inodestart: 1,
    ndatablocks: 5,
    bmapstart: 4,
    datastart: 5,
};

static SUPERBLOCK_BAD_ORDER: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE,
    nblocks: NBLOCKS,
    ninodes: 1000,
    inodestart: 1,
    ndatablocks: 5,
    bmapstart: 5,
    datastart: 6,
};

fn disk_prep_path(name: &str) -> PathBuf {
    utils::disk_prep_path(&("fs-images-a-".to_string() + name), "img")
}

//Create a fresh device
fn disk_setup(path: &Path) -> Device {
    utils::disk_setup(path, BLOCK_SIZE, NBLOCKS)
}

#[test]
fn mkfs() {
    let path = disk_prep_path("mkfs");
    //Some failing mkfs calls
    assert!(FSName::mkfs(&path, &SUPERBLOCK_BAD_INODES).is_err());
    assert!(FSName::mkfs(&path, &SUPERBLOCK_BAD_ORDER).is_err());

    //A working one
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();
    let sb = my_fs.b_get(0).unwrap();
    assert_eq!(
        sb.deserialize_from::<SuperBlock>(0).unwrap(),
        SUPERBLOCK_GOOD
    );
    assert_eq!(my_fs.sup_get().unwrap(), SUPERBLOCK_GOOD);
    my_fs.sup_put(&SUPERBLOCK_BAD_INODES).unwrap();
    assert_eq!(my_fs.sup_get().unwrap(), SUPERBLOCK_BAD_INODES);

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn mountfs_f() {
    let path = disk_prep_path("mountfs_f");
    let mut dev = disk_setup(&path);

    let mut sb = dev.read_block(0).unwrap();
    sb.serialize_into(&SUPERBLOCK_BAD_INODES, 0).unwrap();
    dev.write_block(&sb).unwrap();

    assert!(FSName::mountfs(dev).is_err());

    utils::disk_unprep_path(&path); //This call should be ok here; the device has already been dropped
}

#[test]
fn mountfs_s() {
    let path = disk_prep_path("mountfs_s");
    let mut dev = disk_setup(&path);

    let mut sb = dev.read_block(0).unwrap();
    sb.serialize_into(&SUPERBLOCK_GOOD, 0).unwrap();
    dev.write_block(&sb).unwrap();

    let my_fs = FSName::mountfs(dev).unwrap();
    let sb = my_fs.b_get(0).unwrap();
    assert_eq!(
        sb.deserialize_from::<SuperBlock>(0).unwrap(),
        SUPERBLOCK_GOOD
    );

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn cod_block_ops() {
    let path = disk_prep_path("block_ops");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let zb = |i| utils::zero_block(i, BLOCK_SIZE);
    for i in 1..NBLOCKS {
        //Will fail if you sneak in inodesupport
        assert_eq!(my_fs.b_get(i).unwrap(), zb(i));
    }

    let nb = utils::n_block(5, BLOCK_SIZE, 6);
    my_fs.b_put(&nb).unwrap();
    let b = my_fs.b_get(5).unwrap();
    assert_eq!(b, nb);

    let nb_bis = utils::n_block(6, BLOCK_SIZE, 6);
    my_fs.b_put(&nb_bis).unwrap();
    my_fs.b_zero(1).unwrap(); //zero this block again
    let b = my_fs.b_get(6).unwrap();
    assert_eq!(b, zb(6));

    assert!(my_fs.b_zero(5).is_err()); //out of bounds

    let dev = my_fs.unmountfs();
    assert_eq!(dev.read_block(5).unwrap(), nb); //make sure you actually persisted stuff

    utils::disk_destruct(dev);
}

//Note that this test does not check the case where multiple bit blocks have to be loaded from memory, or if the resulting block is zeroed
#[test]
fn free_alloc() {
    let path = disk_prep_path("free_alloc");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();
    let mut byte: [u8; 1] = [0];

    //Allocate
    for i in 0..SUPERBLOCK_GOOD.ndatablocks {
        assert_eq!(my_fs.b_alloc().unwrap(), i); //Fill up all data blocks
    }
    assert!(my_fs.b_alloc().is_err()); //No more blocks

    //Check the bitmap
    let bb = my_fs.b_get(4).unwrap();
    bb.read_data(&mut byte, 0).unwrap();
    assert_eq!(byte[0], 0b0001_1111);

    //Deallocate
    my_fs.b_free(3).unwrap();
    assert!(my_fs.b_free(3).is_err());

    //Check the bitmap
    let bb = my_fs.b_get(4).unwrap();
    bb.read_data(&mut byte, 0).unwrap();
    assert_eq!(byte[0], 0b0001_0111);

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}
