//! A library for encoding/decoding Unix archive files.
//!
//! This library provides utilities necessary to manage [Unix archive
//! files](https://en.wikipedia.org/wiki/Ar_(Unix)) (as generated by the
//! standard `ar` command line utility) abstracted over a reader or writer.
//! This library provides a streaming interface that avoids having to ever load
//! a full archive entry into memory.
//!
//! The API of this crate is meant to be similar to that of the
//! [`tar`](https://crates.io/crates/tar) crate.
//!
//! # Format variants
//!
//! Unix archive files come in several variants, of which three are the most
//! common:
//!
//! * The *common variant*, used for Debian package (`.deb`) files among other
//!   things, which only supports filenames up to 16 characters.
//! * The *BSD variant*, used by the `ar` utility on BSD systems (including Mac
//!   OS X), which is backwards-compatible with the common variant, but extends
//!   it to support longer filenames and filenames containing spaces.
//! * The *GNU variant*, used by the `ar` utility on GNU and many other systems
//!   (including Windows), which is similar to the common format, but which
//!   stores filenames in a slightly different, incompatible way, and has its
//!   own strategy for supporting long filenames.
//!
//! This crate supports reading and writing all three of these variants.
//!
//! # Example usage
//!
//! Writing an archive:
//!
//! ```no_run
//! use ar::Builder;
//! use std::fs::File;
//! // Create a new archive that will be written to foo.a:
//! let mut builder = Builder::new(File::create("foo.a").unwrap());
//! // Add foo/bar.txt to the archive, under the name "bar.txt":
//! builder.append_path("foo/bar.txt").unwrap();
//! // Add foo/baz.txt to the archive, under the name "hello.txt":
//! let mut file = File::open("foo/baz.txt").unwrap();
//! builder.append_file(b"hello.txt", &mut file).unwrap();
//! ```
//!
//! Reading an archive:
//!
//! ```no_run
//! use ar::Archive;
//! use std::fs::File;
//! use std::io;
//! use std::str;
//! // Read an archive from the file foo.a:
//! let mut archive = Archive::new(File::open("foo.a").unwrap());
//! // Iterate over all entries in the archive:
//! while let Some(entry_result) = archive.next_entry() {
//!     let mut entry = entry_result.unwrap();
//!     // Create a new file with the same name as the archive entry:
//!     let mut file = File::create(
//!         str::from_utf8(entry.header().identifier()).unwrap(),
//!     ).unwrap();
//!     // The Entry object also acts as an io::Read, so we can easily copy the
//!     // contents of the archive entry into the file:
//!     io::copy(&mut entry, &mut file).unwrap();
//! }
//! ```

#![warn(missing_docs)]

pub use crate::archive::{Archive, Variant};
pub use crate::builder::Builder;
pub use crate::builder::GnuBuilder;
pub use crate::entry::Entry;
pub use crate::header::Header;
pub use crate::symbols::Symbols;

mod archive;
mod builder;
mod entry;
mod error;
mod header;
mod symbols;
