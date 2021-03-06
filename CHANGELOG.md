# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
- Unreleased changes will be listed herei.

## [0.5.1] - 2019-02-24
### Changed
- **Breaking Change**: `Data::load()` now takes one argument as the file name.
- Use Workflow's cache directory for storing temp. files
### Added
- Add a clear() method to Data struct.

## [0.4.3] - 2019-02-22
### Fixed
- Fix crate version for docs.rs

## [0.4.0] - 2018-07-04
### Added
- **Breaking changes**
- Methods that save data now accept `ref` instead of moving the value to be save.
### Fixed
- Checking for updates will now correctly make network calls after prior failures.
