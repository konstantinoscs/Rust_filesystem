//! File system with support for inode caching\
//! Reimplementation of the inodes from the base project.
//!
//! Create a filesystem that has a notion of inodes and blocks and allows you to have a certain number of inodes in an inode cache, by implementing the [`FileSysSupport`], the [`BlockSupport`], the [`InodeSupport`] and the [`InodeRWSupport`] and [`InodeCacheSupport`] traits together (again, all earlier traits are supertraits of the last two).
//!
//! [`FileSysSupport`]: ../../cplfs_api/fs/trait.FileSysSupport.html
//! [`BlockSupport`]: ../../cplfs_api/fs/trait.BlockSupport.html
//! [`InodeSupport`]: ../../cplfs_api/fs/trait.InodeSupport.html
//! [`InodeRWSupport`]: ../../cplfs_api/fs/trait.InodeRWSupport.html
//! [`InodeCacheSupport`]: ../../cplfs_api/fs/trait.InodeCacheSupport.html
//!
//! You have to support caching of inodes through the use of an *inode cache*, i.e. we want to make sure that:
//! - Inodes that have been read before do not have to be read from the disk again, if they are still in the cache. This is a performance improvement.
//! - If the same inode is read from the disk multiple, and multiple copies of it are in use at the same time, then each one of these copies refers to the *same* inode, and not to independent, owned instances of it, as was the case in the base project. This is a usability improvement. In other words, if one inode is read from the disk into the cache and a reference to this cache entry is kept in the code in different locations, changes to the inode in one location should be visible in all other locations, i.e. clients of our code do not have to be as careful anymore updating inodes.
//!
//! Caching provides some practical issues in Rust, given that the type system does not allow us to have multiple mutable references to a single cache entry at the same time.
//! The naive solution where `i_get` returns some form of mutable reference to an inode in the cache hence does not work.
//! Essentially, the problem is that it is impossible to know statically that the cache entries will be used in a safe manner.
//! Clearly, we have to enforce ownership and borrowing at runtime.
//! One possible way of doing this consists of two parts:
//! - Use a `RefCell` to make sure that borrowing rules are only checked at runtime, i.e. use the `borrow` and `borrow_mut` methods on the `RefCell` type to perform borrow checking at runtime. `RefCell` allows for *interior mutability*, in the sense that a regular reference to a value of type `RefCell` still allows its contents to be mutated. This is safe, since `RefCell` checks the borrowing rules at runtime regardless.
//! Read more about this [here](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html).
//! - `RefCell` has some limitations; it still only allows a single party to have ownership of its values. This will not suffice if we want to keep multiple copies of a cached entry in memory. To this end, we can wrap our `RefCell`s in the `Rc` (reference count) type; this type allows us to have multiple (immutable) copies of the value it wraps. A `Rc` value keeps track of the number of owners at each point in time, and will only free its contents when the last owner goes out of scope. Read more about this [here](https://doc.rust-lang.org/book/ch15-04-rc.html). This type interacts very nicely with `RefCell`, since an immutable reference suffices to be allowed to mutate the `RefCell`'s contents.
//!
//! Using a combination of these two types, we can now create a shareable wrapper for our original inode type as follows:
//! ```ignore
//!use std::cell::RefCell;
//!use std::rc::Rc;
//! #[derive(Debug, Default, PartialEq, Eq, Clone)]
//! pub struct InodeCached(Rc<RefCell<Inode>>);
//! ```
//! You should still implement `InodeLike` so that you can use this wrapper in your trait implementations. Additionally, think of some useful helper methods to define on this type.
//!
//! If we create a fixed-size cache data structure (pick an appropriate type for this cache in your implementation) that contains entries of this `InodeCached` type, we can actually hand out multiple copies (by cloning the `Rc` value, this implementation of `clone` is how `InodeCached` is able to derive the `Clone` trait in the above code), and make sure that they are used safely (thanks to the dynamic checking of `RefCell`).
//! This solution is still not entirely realistic, as the cache's contents will be dynamically allocated and spread across the heap when we create new `InodeCached` instances from `Inodes`, but it is already a big step in the right direction.
//!
//! Now it is your turn. Implement the aforementioned cache structure, add it to your previous filesystem implementation with inodes, and make sure to implement the `InodeCacheSupport` trait.
//! Additionally, go back and fix the implementations of the functions in the `InodeSupport` to make them aware of our caching schema.
//! The following changes are required to the functions that you implemented before as part of the `InodeSupport` trait:
//! - `i_get` takes an immutable reference to `self`, and will hence be incapable of making any changes to the cache. For this reason, the `InodeCacheSupport` trait provides a new method `i_get_mut`, which takes a mutable reference to self, and hence allows updating the cache as part of the read process. More concretely `i_get` will look for an inode entry in the inode cache only, return a reference to it if it finds it and error otherwise. On the other hand, `i_get_mut` will first look in the cache and copy the behavior of `i_get`, but rather than returning an error on lookup failure, read the inode number from the disk instead. See the documentation of `i_get_mut` for more information.
//! - `i_put` still takes a reference to an inode and writes it back to the disk. The only difference is that the provided reference is now a reference to a cached inode, but this should not matter much for your implementation
//! - `i_free`: the new implementation of `i_free` differs from the old implementation (without caching) like `i_get_mut` differs from `i_get`.
//! The new implementation first tries to free the inode `i` from the cache. If the node is found, the following happens:
//!     - Returns an error if the node is still referenced elsewhere (again, you can check this through the `strong_count` method on the `Rc` type)
//!     - Does nothing and returns with an `Ok` if there are other links to this inode still (as was the case before)
//!     - Errors when trying to free an already free inode (as was the case before)
//!     - If the previous 3 cases do not occur, we can actually free the inode, as specified in `i_free`. Make sure the freed inode is written back to disk in the end.
//! If the inode is not cached, the disk inode is fetched from disk (*WARNING*: this disk inode should **NOT** end up in the cache, as we are about to free it anyways). The previous checks are then repeated, and the freed disk inode is persisted.
//! - One change to `i_alloc` is that the allocated inode will now be read into the cache too (but not returned), replacing a pre-existing free entry for the same inode if necessary.
//! We have to do this to avoid a remaining free entry in the cache for the allocated inode shadowing our allocated entry on disk. The implementation of `i_alloc` can remain otherwise unchanged, because of the following invariant of our system: *no free nodes will ever be mutated in the cache*. In other words, if `i_alloc` encounters a free inode on disk, it knows that there should not be a non-free version of this inode in the cache. This allows the implementation of `i_alloc` to disregard the cache contents.
//! - `i_trunc`, `i_read` and `i_write` do not change substantially.
//!
//! At the end, write some tests that convincingly show that your implementation indeed supports cached inodes.
//!
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

/// You are free to choose the name for your file system. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your file system data type so we can just use `FSName` instead of
/// having to manually figure out the name.
/// **TODO**: replace the below type by the type of your file system
pub type FSName = ();

// **TODO** define your own tests here.

// WARNING: DO NOT TOUCH THE BELOW CODE -- IT IS REQUIRED FOR TESTING -- YOU WILL LOSE POINTS IF I MANUALLY HAVE TO FIX YOUR TESTS
#[cfg(all(test, any(feature = "g", feature = "all")))]
#[path = "../../api/fs-tests/g_test.rs"]
mod tests;
