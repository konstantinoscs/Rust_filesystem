use std::io;
use thiserror::Error;

///Error type used in the provided code
/// (See the code to understand the following explanation, and compare the code to what is output in the documentation)
/// The `#[error]` tag effectively takes care of the `Display` aspect of your errors, generating specific cases in the implicitly derived implementation of the `Display` trait.
/// The `#[from]` tag allows us to wrap a different type error in this error. This automatically generates a `From`-trait implementation, allowing conversion from `io::Error`s to `ControllerIO`-errors when using the `?` operator, as you can see in the code of e.g. [`controller.rs`](../controller.html)
#[derive(Error, Debug)]
pub enum APIError {
    /// Error caused when performing IO in the API
    #[error("Issue using IO in the controller layer")]
    APIO(#[from] io::Error),
    /// Error caused when performing IO in the API
    //#[error("Issue with serialization in the controller layer")]
    //APISerialize(#[from] bincode::Error),
    /// Invalid input to the controller layer
    /// Note: use `String` instead of `&'static str` if you want non-literal, i.e. non-hard-coded, runtime error messages
    #[error("Invalid controller input: {0}")]
    ControllerInput(&'static str),
    /// Invalid input to a block
    #[error("Invalid block input: {0}")]
    BlockInput(&'static str)
    /*
    ///*EXTRA:* *Avoid* using this catch-all error in your own submission, as it is not practical to handle
    ///The [`anyhow`](https://docs.rs/anyhow/1.0.33/anyhow/) package allows defining universal error types, that any error can be cast into
    ///This package allows using the macro `anyhow!` to transform any error implementing the `Error` trait into an `anyhow` error, and interacts nicely with the `?` operator.
    ///This is great for easy and uniform error reporting and return types of client code, but not when you want to handle errors afterwards, as it is now impossible to match on individual error variants
    ///This error has mostly been added for illustrative purposes, and can be useful for quickly drafting some code without thinking about the concrete error instances you want
    #[error(transparent)]
    Other(#[from] anyhow::Error), // source and Display delegate to anyhow::Error */*/
}

/// Define a generic alias for a `Result` with the error type `APIError`.
/// This shorthand is what I use in my implementation to define error types
pub type Result<T> = std::result::Result<T, APIError>;