// Copyright (c) 2018 Hamid R. Ghadyani.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helpers for writing Alfred workflows.
//!
//! ## Features:
//! - Self automatic update of workflows using this crate ([`updater`]).
//! - Read/write API for workflow data (settings, cache data, ...) ([`data`]).
//!
//! This crate adds enhanced features and quality-of-life improvements to
//! [other alfred crate][alfred]'s basic functionality of creating **Script Filter** items.
//!
//! [`updater`]: updater/index.html
//! [`data`]: data/index.html
//! [alfred]: https://crates.io/crates/alfred
//!
#![doc(html_root_url = "https://docs.rs/alfred-rs/0.3.1")]

extern crate alfred;
extern crate failure;
extern crate serde;
extern crate serde_json;

#[cfg(test)]
extern crate mockito;
#[cfg(test)]
extern crate tempfile;

extern crate chrono;
extern crate reqwest;
extern crate semver;
#[macro_use]
extern crate serde_derive;
extern crate time;
extern crate url;
extern crate url_serde;

use alfred::env;
use failure::err_msg;
use failure::Error;

pub mod data;
pub mod updater;

pub use self::data::Data;
pub use self::updater::Updater;
