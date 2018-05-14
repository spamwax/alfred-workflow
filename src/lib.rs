// Copyright (c) 2018 Hamid R. Ghadyani.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helpers for writing Alfred script filter output
//! See [`updater`] module documentation for details and examples.
//!
//! [`updater`]: updater/index.html
//!

extern crate alfred;
extern crate failure;
extern crate semver;
extern crate serde;
extern crate serde_json;

#[cfg(test)]
extern crate mockito;
#[cfg(test)]
extern crate tempfile;

#[cfg(feature = "updater")]
extern crate chrono;
#[cfg(feature = "updater")]
extern crate reqwest;
#[cfg(feature = "updater")]
#[macro_use]
extern crate serde_derive;
#[cfg(feature = "updater")]
extern crate time;
#[cfg(feature = "updater")]
extern crate url;
#[cfg(feature = "updater")]
extern crate url_serde;

#[cfg(feature = "updater")]
pub mod updater;

#[cfg(feature = "updater")]
pub use self::updater::Updater;

use alfred::env;
