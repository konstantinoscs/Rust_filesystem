#![allow(dead_code)]

//Some more general testing utilities
use cplfs_api::controller::Device;
use cplfs_api::types::Block;
use std::fs::{create_dir_all, remove_dir, remove_file};
use std::path::{Path, PathBuf};

//Create the necessary folders 'name' leading up to 'img_name', starting from the crae root
//Additionally, remove 'img_name' if it already exists in the file system, to make sure we can start from a fresh disk
pub fn disk_prep_path(name: &str, img_name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(name);
    path.push(img_name);

    if path.exists() {
        //Remove the file in case it already exists
        remove_file(&path).unwrap();
    }
    {
        //Create any missing directories first, if applicable
        let prefix = path.parent().unwrap();
        create_dir_all(prefix).unwrap();
    }

    return path;
}

//Undo folder creation, including removing the parent
pub fn disk_unprep_path(path: &Path) {
    //Ensure that the file has been deleted before going on
    remove_file(path).unwrap();

    let parent = path.parent().unwrap();
    remove_dir(parent).unwrap(); //Safety; only remove if empty
}

//Create a fresh device
pub fn disk_setup(path: &Path, block_size: u64, nblocks: u64) -> Device {
    Device::new(path, block_size, nblocks).unwrap()
}

//Create an existing device
pub fn disk_open(path: &Path, block_size: u64, nblocks: u64) -> Device {
    Device::load(path, block_size, nblocks).unwrap()
}

//Destruct the given device and remove the parent directory that is was located in
pub fn disk_destruct(dev: Device) {
    let path = dev.device_path().to_owned();
    drop(dev); //Avoid the device holding a lock over this file
    disk_unprep_path(&path);
}

//Create a block consisting of all zeroes
pub fn zero_block(block_no: u64, block_size: u64) -> Block {
    n_block(block_no, block_size, 0)
}

//Create a block consisting of all n
pub fn n_block(block_no: u64, block_size: u64, n: u8) -> Block {
    Block::new(block_no, vec![n; block_size as usize].into_boxed_slice())
}
