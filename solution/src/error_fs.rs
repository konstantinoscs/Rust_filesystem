use std::io;
use thiserror::Error;
use cplfs_api::error_given::APIError;

///Error type used in the provided code
/// (See the code to understand the following explanation, and compare the code to what is output in the documentation)
/// The `#[error]` tag effectively takes care of the `Display` aspect of your errors, generating specific cases in the implicitly derived implementation of the `Display` trait.
/// The `#[from]` tag allows us to wrap a different type error in this error. This automatically generates a `From`-trait implementation, allowing conversion from `io::Error`s to `ControllerIO`-errors when using the `?` operator, as you can see in the code of e.g. [`controller.rs`](../controller.html)
#[derive(Error, Debug)]
pub enum BlockLayerError {
    ///errors from the controller layer
    #[error("Error in the controller layer")]
    ControllerError(#[from] APIError),

    /// errors regarding input on the BLockLayerFS
    #[error("Error in the input of BLockLayerFS: {0}")]
    BlockLayerInput(&'static str)
}

/// Define a generic alias for a `Result` with the error type `APIError`.
/// This shorthand is what I use in my implementation to define error types
pub type Result<T> = std::result::Result<T, BlockLayerError>;