# alfred-rs

[![alfred-rs](https://github.com/spamwax/alfred-workflow/actions/workflows/main.yml/badge.svg)](https://github.com/spamwax/alfred-workflow/actions/workflows/main.yml)
[![alfred-rs](https://github.com/spamwax/alfred-workflow/actions/workflows/lint.yml/badge.svg)](https://github.com/spamwax/alfred-workflow/actions/workflows/lint.yml)
[![crates.io/crates/alfred-rs](https://img.shields.io/crates/v/alfred-rs)](https://crates.io/crates/alfred-rs)
[![https://docs.rs/alfred-rs/badge.svg](https://docs.rs/alfred-rs/badge.svg)](https://docs.rs/crate/alfred-rs/)

Write [Workflows][] for [Alfred][alfred.app] app with ease!

This crate adds enhanced features and quality-of-life improvements to
[other alfred crate][alfred]'s basic functionality of generating items
for **Script Filter** types in Alfred.

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

## Changelog

Change logs are now kept in a [separate document](./CHANGELOG.md).

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT) at your option.
