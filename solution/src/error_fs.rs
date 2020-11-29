use cplfs_api::error_given::APIError;
use thiserror::Error;

///Error type used in the BlockLayer
#[derive(Error, Debug)]
pub enum BlockLayerError {
    ///errors from the controller layer
    #[error("Error in the controller layer")]
    ControllerError(#[from] APIError),

    /// errors regarding input on the BLockLayerFS
    /// these errors are thrown when there is a problem while reading something
    #[error("Error in the input of BLockLayerFS: {0}")]
    BlockLayerInput(&'static str),

    ///errors regarding problems when writing
    #[error("Invalid write in BLockLayerFS: {0}")]
    BlockLayerWrite(&'static str),

    ///errors regarding the internal state of the FS
    #[error("Error in operation of BlockLayerFS: {0}")]
    BlockLayerOp(&'static str),
}

///Error type used in the InodeLayer
#[derive(Error, Debug)]
pub enum InodeLayerError {
    ///errors from the controller layer
    #[error("Error in the controller layer")]
    ControllerError(#[from] APIError),

    ///errors from the block layer
    #[error("Error in the block layer")]
    BlockLayerError(#[from] BlockLayerError),

    /// errors regarding input on the InodeLayerFS
    #[error("Error in the input of InodeLayerFS: {0}")]
    InodeLayerInput(&'static str),

    ///errors regarding the internal state of the FS
    #[error("Error in operation of InodeLayerFS: {0}")]
    InodeLayerOp(&'static str),
}

///Error type used in the DirLayer
#[derive(Error, Debug)]
pub enum DirLayerError {
    ///errors from the block layer
    #[error("Error in the Inode layer")]
    InodeLayerError(#[from] InodeLayerError),
}

/*/// Define a generic alias for a `Result` with the error type `APIError`.
/// This shorthand is what I use in my implementation to define error types*/
//pub type Result<T> = std::result::Result<T, BlockLayerError>;
