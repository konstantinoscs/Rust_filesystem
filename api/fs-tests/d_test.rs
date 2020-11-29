use super::FSName;
use cplfs_api::fs::{BlockSupport, DirectorySupport, FileSysSupport, InodeSupport, PathSupport};
use cplfs_api::types::{FType, InodeLike, SuperBlock, DIRENTRY_SIZE};
use std::path::PathBuf;

#[path = "utils.rs"]
mod utils;

static BLOCK_SIZE: u64 = 1000;
static NBLOCKS: u64 = 12;
static SUPERBLOCK_GOOD: SuperBlock = SuperBlock {
    block_size: BLOCK_SIZE,
    nblocks: NBLOCKS,
    ninodes: 8,
    inodestart: 1,
    ndatablocks: 7,
    bmapstart: 4,
    datastart: 5,
};

fn disk_prep_path(name: &str) -> PathBuf {
    utils::disk_prep_path(&("fs-images-d-".to_string() + name), "img")
}

#[test]
fn cwd_utils() {
    let path = disk_prep_path("cwd");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    assert_eq!(&my_fs.get_cwd(), "/");

    assert!(!FSName::valid_path(""));
    assert!(!FSName::valid_path("//"));
    assert!(!FSName::valid_path("a"));
    assert!(!FSName::valid_path("/a/")); //we do not allow ending on a "/", as we interpret this as the last entry being empty
    assert!(!FSName::valid_path("/❤"));
    assert!(!FSName::valid_path("/fartoolongtobevalid"));
    assert!(FSName::valid_path("/some/regular/name/.."));

    let path_0 = "../../test";
    my_fs.set_cwd(path_0).unwrap();
    assert_eq!(&my_fs.get_cwd(), "/test");

    let path_rel = "../testrel/test2/../test4";
    my_fs.set_cwd(path_rel).unwrap();
    assert_eq!(&my_fs.get_cwd(), "/testrel/test4");
    let path_a = "/testa/./test2/../test3";
    my_fs.set_cwd(path_a).unwrap();
    assert_eq!(&my_fs.get_cwd(), "/testa/test3");

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn root() {
    let path = disk_prep_path("root");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let mut iroot = my_fs.i_get(1).unwrap();
    assert_eq!(iroot.get_ft(), FType::TDir);
    assert_eq!(iroot.get_nlink(), 1);
    assert_eq!(iroot.get_size(), 2 * *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&mut iroot, ".").unwrap().1, 0);
    assert_eq!(my_fs.dirlookup(&mut iroot, ".").unwrap().0.get_inum(), 1);
    assert_eq!(my_fs.dirlookup(&mut iroot, "..").unwrap().1, *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&mut iroot, "..").unwrap().0.get_inum(), 1);
    //The root should have been allocated a block for its data
    my_fs.b_free(0).unwrap();

    assert_eq!(my_fs.i_get(0).unwrap().get_ft(), FType::TFree);
    my_fs.i_free(1).unwrap(); //inode has been allocated, but should not be deallocated, as the root references itself
    my_fs.i_free(1).unwrap(); //so this should work twice
    assert!(my_fs.i_free(0).is_err()); //inode has not been allocated

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn resolve() {
    let path = disk_prep_path("resolve");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let mut iroot = my_fs.i_get(1).unwrap();

    //Allocate blocks 6-7-8
    for i in 0..3 {
        assert_eq!(my_fs.b_alloc().unwrap(), i + 1);
    }
    //Setup inode 3, using these blocks
    let i3 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        3,
        &FType::TDir,
        0,
        (2.5 * (BLOCK_SIZE as f32)) as u64,
        &[6, 7, 8],
    )
    .unwrap();
    my_fs.i_put(&i3).unwrap();

    //Setup inode 5
    let i5 =
        <<FSName as InodeSupport>::Inode as InodeLike>::new(5, &FType::TDir, 0, 0, &[]).unwrap();
    my_fs.i_put(&i5).unwrap();

    //Setting up some links between files
    //Creating a loop so we can make it somewhat longer
    assert_eq!(
        my_fs.dirlink(&mut iroot, "in13", 3).unwrap(),
        2 * *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs.dirlink(&mut iroot, "in15", 5).unwrap(),
        3 * *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs.dirlink(&mut iroot, "in13bis", 3).unwrap(),
        4 * *DIRENTRY_SIZE
    );
    //WARNING; be careful with these tests => inodes are no longer up to date when you start writing -> read back from disk to stay up to date
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(5).unwrap(), "in53", 3)
            .unwrap(),
        0
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(5).unwrap(), "in51", 1)
            .unwrap(),
        *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "in35", 5)
            .unwrap(),
        0
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "in31", 1)
            .unwrap(),
        *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "in33", 3)
            .unwrap(),
        2 * *DIRENTRY_SIZE
    );

    //Little hack: hardcode parent dirs here
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(5).unwrap(), "..", 1)
            .unwrap(),
        2 * *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs.dirlink(&mut my_fs.i_get(5).unwrap(), ".", 5).unwrap(),
        3 * *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "..", 1)
            .unwrap(),
        3 * *DIRENTRY_SIZE
    );

    //Now you can follow the resolution using the above links
    assert_eq!(
        my_fs
            .resolve_path("./in13/.././in13bis/../../in15")
            .unwrap()
            .get_inum(),
        5
    );
    assert_eq!(
        my_fs
            .resolve_path("/in13/in35/in51/../in13")
            .unwrap()
            .get_inum(),
        3
    );

    my_fs.set_cwd("/in15/../in31").unwrap(); //Does not match entries -> should go wrong
    assert!(my_fs.resolve_path("./in13").is_err());

    my_fs.set_cwd("/in15/../in15").unwrap();
    assert_eq!(my_fs.resolve_path("./in53/../in13").unwrap().get_inum(), 3);
    my_fs.set_cwd("/in15/../in15").unwrap();
    assert_eq!(my_fs.resolve_path("../in13").unwrap().get_inum(), 3);

    //Not a directory
    //Re-setup inode 3
    let i3 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        3,
        &FType::TFile,
        3,
        (2.5 * (BLOCK_SIZE as f32)) as u64,
        &[6, 7, 8],
    )
    .unwrap();
    my_fs.i_put(&i3).unwrap();
    my_fs.set_cwd("/").unwrap();
    assert!(my_fs.resolve_path("./in13/in35").is_err()); //3 is not dir

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn mkdir() {
    let path = disk_prep_path("mkdir");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let mut iroot = my_fs.i_get(1).unwrap();

    //Allocate blocks 6-7-8
    for i in 0..3 {
        assert_eq!(my_fs.b_alloc().unwrap(), i + 1);
    }
    //Setup inode 3, using these blocks
    let i3 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        3,
        &FType::TDir,
        0,
        (2.5 * (BLOCK_SIZE as f32)) as u64,
        &[6, 7, 8],
    )
    .unwrap();
    my_fs.i_put(&i3).unwrap();

    //Setup inode 5
    let i5 =
        <<FSName as InodeSupport>::Inode as InodeLike>::new(5, &FType::TDir, 0, 0, &[]).unwrap();
    my_fs.i_put(&i5).unwrap();

    //Setting up some links between files
    assert_eq!(
        my_fs.dirlink(&mut iroot, "in13", 3).unwrap(),
        2 * *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "in35", 5)
            .unwrap(),
        0
    );
    //Little hack: hardcode parent dirs here
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "..", 1)
            .unwrap(),
        *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs.dirlink(&mut my_fs.i_get(5).unwrap(), ".", 5).unwrap(),
        0
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(5).unwrap(), "..", 3)
            .unwrap(),
        *DIRENTRY_SIZE
    );

    //nlink is 1, from 3
    assert_eq!(my_fs.i_get(5).unwrap().get_nlink(), 1);
    //nlink is 2, from 1 and 5
    assert_eq!(my_fs.i_get(3).unwrap().get_nlink(), 2);

    assert!(my_fs.mkdir("./in13/in35/.").is_err());

    //Create a directory in inode 5
    let i_new = my_fs.mkdir("./in13/in35/testie").unwrap();
    assert_eq!(i_new, my_fs.i_get(2).unwrap());
    assert_eq!(i_new.get_nlink(), 1);
    assert_eq!(i_new.get_size(), 2 * *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&i_new, "..").unwrap().1, *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&i_new, ".").unwrap().0.inum, 2);
    assert_eq!(my_fs.dirlookup(&i_new, "..").unwrap().0.inum, 5);

    my_fs.set_cwd("/in13/../in13/in35").unwrap();
    assert_eq!(
        my_fs
            .resolve_path("./testie/../testie/.")
            .unwrap()
            .get_inum(),
        2
    );

    //Create a directory in testie
    let i_new2 = my_fs.mkdir("../../in13/in35/testie/testie2").unwrap();
    assert_eq!(i_new2, my_fs.i_get(4).unwrap());
    assert_eq!(i_new2.get_nlink(), 1);
    assert_eq!(my_fs.i_get(2).unwrap().get_nlink(), 2); //Link to testie
    assert_eq!(i_new2.get_size(), 2 * *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&i_new2, "..").unwrap().1, *DIRENTRY_SIZE);
    assert_eq!(my_fs.dirlookup(&i_new2, ".").unwrap().0.inum, 4);
    assert_eq!(my_fs.dirlookup(&i_new2, "..").unwrap().0.inum, 2);

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

#[test]
fn unlink() {
    let path = disk_prep_path("unlink");
    let mut my_fs = FSName::mkfs(&path, &SUPERBLOCK_GOOD).unwrap();

    let mut iroot = my_fs.i_get(1).unwrap();

    //Allocate blocks 6-7-8
    for i in 0..3 {
        assert_eq!(my_fs.b_alloc().unwrap(), i + 1);
    }
    //Setup inode 3, using these blocks
    let i3 = <<FSName as InodeSupport>::Inode as InodeLike>::new(
        3,
        &FType::TDir,
        0,
        (2.5 * (BLOCK_SIZE as f32)) as u64,
        &[6, 7, 8],
    )
    .unwrap();
    my_fs.i_put(&i3).unwrap();

    //Setup inode 5
    let i5 =
        <<FSName as InodeSupport>::Inode as InodeLike>::new(5, &FType::TDir, 0, 0, &[]).unwrap();
    my_fs.i_put(&i5).unwrap();

    //Setting up some links between files
    assert_eq!(
        my_fs.dirlink(&mut iroot, "in13", 3).unwrap(),
        2 * *DIRENTRY_SIZE
    );
    //Little hack: hardcode special dirs here post-factum, so that sizes work out
    assert_eq!(
        my_fs.dirlink(&mut my_fs.i_get(3).unwrap(), ".", 3).unwrap(),
        0
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "..", 1)
            .unwrap(),
        *DIRENTRY_SIZE
    );
    assert_eq!(
        my_fs.dirlink(&mut my_fs.i_get(5).unwrap(), ".", 5).unwrap(),
        0
    );
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(5).unwrap(), "..", 3)
            .unwrap(),
        *DIRENTRY_SIZE
    );
    //Add one extra link
    assert_eq!(
        my_fs
            .dirlink(&mut my_fs.i_get(3).unwrap(), "in35", 5)
            .unwrap(),
        2 * *DIRENTRY_SIZE
    );

    //nlink is 1, from 3
    assert_eq!(my_fs.i_get(5).unwrap().get_nlink(), 1);
    //nlink is 2, from 1 and 5
    assert_eq!(my_fs.i_get(3).unwrap().get_nlink(), 2);

    assert!(my_fs.mkdir("./in13/in35/.").is_err());

    //Create a directory in inode 5
    let i_new = my_fs.mkdir("./in13/in35/testie").unwrap();
    assert_eq!(i_new.get_inum(), 2);

    //Some failing attempts - filled directories
    assert!(my_fs.unlink("./in13").is_err());
    assert!(my_fs.unlink("./in13/in35").is_err());
    assert!(my_fs.unlink("./in13/in35/testie/.").is_err());
    assert!(my_fs.unlink("./in13/in35/testie/..").is_err());

    my_fs.set_cwd("./in13");

    //Some failing attempts
    assert!(my_fs.unlink("./in35").is_err());

    //Link testie a second time
    my_fs
        .dirlink(&mut my_fs.i_get(5).unwrap(), "testiebis", 2)
        .unwrap();

    my_fs.dirlookup(&my_fs.i_get(5).unwrap(), "testie").unwrap(); //allocated
    my_fs
        .dirlookup(&my_fs.i_get(5).unwrap(), "testiebis")
        .unwrap(); //allocated
    assert_eq!(my_fs.i_get(2).unwrap().get_nlink(), 2);
    assert_eq!(my_fs.i_get(5).unwrap().get_nlink(), 2);
    my_fs.unlink("./in35/testie").unwrap();
    assert_eq!(my_fs.i_get(2).unwrap().get_nlink(), 1);
    assert_eq!(my_fs.i_get(5).unwrap().get_nlink(), 2);
    my_fs.unlink("./in35/testiebis").unwrap();
    assert_eq!(my_fs.i_get(2).unwrap().get_nlink(), 0); //freed?
    assert_eq!(my_fs.i_get(2).unwrap().get_ft(), FType::TFree); //freed?
    assert!(my_fs.i_free(2).is_err());
    assert_eq!(my_fs.i_get(5).unwrap().get_nlink(), 1);
    my_fs.unlink("./in35").unwrap();
    assert_eq!(my_fs.i_get(5).unwrap().get_ft(), FType::TFree); //freed?

    let dev = my_fs.unmountfs();
    utils::disk_destruct(dev);
}

//Check that deleting a cyclic reference does not decrease the refcount
