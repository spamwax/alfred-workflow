# alfred-rs

[![Build Status](https://travis-ci.org/spamwax/alfred-workflow.svg?branch=master)](https://travis-ci.org/spamwax/alfred-workflow)
[![crates.io/crates/alfred-rs](http://meritbadge.herokuapp.com/alfred-rs)](https://crates.io/crates/alfred-rs)

Helpers for writing [Alfred][alfred.app] [Workflows][] with features such as:

Features:
- Self automatic update of workflows using this crate.
- Read/write API for workflow data (settings, cache data, ...)

This crate adds enhanced features and quality-of-life improvements to
[other alfred crate][alfred]'s basic functionality of creating **Script Filter** items.

# Note
Currently this crates is in its early stages.

However the [`updater`] is sufficiently stable.

Next planned feature is read/write API.

See [`updater`] module documentation for details and examples.

[`updater`]: https://docs.rs/alfred-rs/latest/alfred_rs/updater/index.html
[alfred]: https://crates.io/crates/alfred
[alfred.app]: http://www.alfredapp.com
[Workflows]: https://www.alfredapp.com/workflows/

[API Documentation](http://docs.rs/alfred-rs)

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]

alfred-rs = "0.2"
```

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT) at your option.
