//! The errors I used in the given code. You can use this file as an inspiration to write down your own error types.
//!
//! Read up on error handling in Rust using the [`error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html ) trait if you haven't already.
//! The below also contains a quick summary.
//!
//! You can define your own enum with different types of errors your filesystem
//! can return. Many functions of this trait will return a
//! `Result<(), Self::Error>`: either an `Ok(())`, meaning that everything
//! went ok, or an `Err(err)` where `err` is of this `Error` type, meaning
//! that an error occurred.
//!
//! Note the two “supertraits” any error type must implement:
//! * `Debug`: a `toString`-like method for debugging that can be derived
//!   automatically using `#[derive(Debug, ...)]`.
//! * `Display`: a `toString`-like method that will be used to show error
//!   to users (as opposed to developers). This method has to be manually
//!   implemented.
//!
//! A first option when implementing errors, is to first define an `Enum` and then implement all necessary traits manually.
//! This might look as follows:
//!
//! # Example: boilerplate implementation
//!
//! ```ignore
//! /// Error type for my window manager
//! #[derive(Debug)]  // Implement `Debug` automatically
//! pub enum APIError {
//!     /// The input provided to some method in the controller layer was invalid
//!     ControllerInvalidInput(String),
//!     // Add more
//!     ...
//! }
//! // Manually implement the `Display` trait.
//! use std::fmt;
//! impl fmt::Display for APIError {
//!     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//!         match *self {
//!             APIError::ControllerInvalidInput(ref s) =>
//!                 write!(f, "Invalid controller input: {}", s),
//!             ...
//!         }
//!     }
//! }
//! // Now implement the `Error` trait.
//! use std::error;
//! impl error::Error for APIError {
//!     //Currently requires no implementation, and will grab the messages from the Display trait
//!     //However, requires more boilerplate code in case we want to wrap errors in other errors and construct a type of backtrace
//!     //In this case, we will have to override the `source` method here (which is what we will implicitly do below)
//! }
//! ```
//!
//! # Example: avoiding error boilerplate code
//!
//! Rather than following the basic template outlined in the documentation, and implementing the `Display` and `Error` traits manually for our error `Enum` as above, we make use of the auxiliary package [`thiserror`](https://docs.rs/thiserror/1.0.21/thiserror/)
//! This package allows us to avoid a lot of the error-related boilerplate code by providing annotations to wrap errors and denote error sources.
//! If each enum variant is annotated with an error-tag, it also allows the `Display` trait to be derived automatically.
//! Error tags support full formatting syntax, and have handy provisions to reference fields of structs.
//! Take a look at the example I wrote down below, and consult the package documentation if you need more details or other variants.
//!
//! If you do not understand this approach very well, you are free *not* to use the thiserror package, and manually embed my errors into yours, or explicitly handle and map them.
//!
//! # Writing your own error types
//! Say you now want to implement your own error type, let's call it `MyError`, and use it as the associated error type in the implementation of the api traits in this assignment.
//! You then have the option of embedding the below error type, allowing for easy interoperability using the `?` operator, by adding e.g. the following variant to your custom error type ;
//! ```ignore
//! #[error(<some_format_string_here>)]
//!  GivenError(#[from] error_given::APIError,...)
//! ```

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
    #[error("Issue with serialization in the controller layer")]
    APISerialize(#[from] bincode::Error),
    /// Invalid input to the controller layer
    /// Note: use `String` instead of `&'static str` if you want non-literal, i.e. non-hard-coded, runtime error messages
    #[error("Invalid controller input: {0}")]
    ControllerInput(&'static str),
    /// Invalid input to a block
    #[error("Invalid block input: {0}")]
    BlockInput(&'static str),

    ///*EXTRA:* *Avoid* using this catch-all error in your own submission, as it is not practical to handle
    ///The [`anyhow`](https://docs.rs/anyhow/1.0.33/anyhow/) package allows defining universal error types, that any error can be cast into
    ///This package allows using the macro `anyhow!` to transform any error implementing the `Error` trait into an `anyhow` error, and interacts nicely with the `?` operator.
    ///This is great for easy and uniform error reporting and return types of client code, but not when you want to handle errors afterwards, as it is now impossible to match on individual error variants
    ///This error has mostly been added for illustrative purposes, and can be useful for quickly drafting some code without thinking about the concrete error instances you want
    #[error(transparent)]
    Other(#[from] anyhow::Error), // source and Display delegate to anyhow::Error
}

/// Define a generic alias for a `Result` with the error type `APIError`.
/// This shorthand is what I use in my implementation to define error types
pub type Result<T> = std::result::Result<T, APIError>;
