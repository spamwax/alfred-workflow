# alfred-rs

[![Build Status](https://travis-ci.org/spamwax/alfred-workflow.svg?branch=master)](https://travis-ci.org/spamwax/alfred-workflow)
[![crates.io/crates/alfred-rs](http://meritbadge.herokuapp.com/alfred-rs)](https://crates.io/crates/alfred-rs)

Write [Workflows][] for [Alfred][alfred.app] app with ease!

This crate adds enhanced features and quality-of-life improvements to
[other alfred crate][alfred]'s basic functionality of creating **Script Filter** items.

Using this crate to create your workflows, you can
- Set up automatic update of workflow ([`updater`] module).
- Painlessly read/write data related to workflow (settings, cache data, ...) ([`data`] module).

## Documentation
For examples and complete documentation visit [API Documentation][].

[`updater`]: https://docs.rs/alfred-rs/latest/alfred_rs/updater/index.html
[`data`]: https://docs.rs/alfred-rs/latest/alfred_rs/data/index.html
[alfred]: https://crates.io/crates/alfred
[alfred.app]: http://www.alfredapp.com
[Workflows]: https://www.alfredapp.com/workflows/
[API Documentation]: http://docs.rs/alfred-rs

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]

alfred-rs = "0.4"
```

## Changelog

- 0.4
  - #### Breaking changes
    - Methods that save data now accept `ref` instead of moving the value to be save.
  - #### Bug fix
    - Checking for updates will now correctly make network calls after prior failures.

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT) at your option.
