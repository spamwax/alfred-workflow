// Copyright (c) 2018 Hamid R. Ghadyani.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Write [Workflows] for [Alfred][alfred.app] app with ease!
//!
//! This crate adds enhanced features and quality-of-life improvements to
//! [other alfred crate][alfred]'s basic functionality of creating **Script Filter** items.
//!
//! Using this crate to create your workflows, you can
//! - Set up automatic update of workflow ([`updater`] module).
//! - Painlessly read/write data related to workflow (settings, cache data, ...) ([`data`] module).
//!
//! [`updater`]: updater/index.html
//! [`data`]: data/index.html
//! [alfred]: https://crates.io/crates/alfred
//! [alfred.app]: http://www.alfredapp.com
//! [Workflows]: https://www.alfredapp.com/workflows/
//!

// TODO: check for "status" field of json returned by github to make sure it is fully uploaded
// before reporting that a release is available.
// TODO: Automatically update html_root_url's version when publishing to crates.io
// TODO: Use https://github.com/softprops/hubcaps for github API?

#![doc(html_root_url = "https://docs.rs/alfred-rs/0.7.1")]

extern crate alfred;
extern crate serde;
extern crate serde_json;

#[cfg(test)]
extern crate mockito;

#[macro_use]
extern crate log;
extern crate chrono;
extern crate env_logger;
extern crate semver;
#[macro_use]
extern crate serde_derive;
extern crate tempfile;
extern crate url;

use alfred::env;
use anyhow::Result;
use anyhow::{anyhow, bail};

pub mod data;
pub mod updater;

pub use self::data::Data;
pub use self::updater::Updater;
