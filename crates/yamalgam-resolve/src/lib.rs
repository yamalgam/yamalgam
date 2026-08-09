//! External resource resolution for Yamalgam.
//!
//! This crate sits above `yamalgam-parser` and below query and CLI consumers.
//! It owns retrieval, resource identity, resolution sessions, and the built-in
//! event middleware without coupling filesystem or network I/O to the parser.
#![deny(unsafe_code)]
