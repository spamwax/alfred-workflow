//! Helper for enabling Alfred workflows to upgrade themselves periodically (Alfred 3)
//!
//! Enable this feature by adding it to your `Cargo.toml`:
//!
//! ```toml
//! alfred = { version = "4", features = ["updater"] }
//! ```
//! Using this module, the workflow author can make Alfred check for latest releases from a remote
//! server within adjustable intervals ([`try_update_ready()`] or [`update_ready()`])
//! (default is 24 hrs).
//!
//! Additionally they can ask Alfred to download the new release to its cache folder for further
//! action [`download_latest()`].
//!
//! For convenience, an associated method [`Updater::gh()`] is available to check
//! for workflows hosted on `github.com`.
//!
//! However, it's possible to check with other servers as long as the [`Releaser`] trait is
//! implemented for the desired remote service.
//! See [`Updater::new()`] documentation if you are hosting your workflow
//! on a non `github.com` service.
//!
//! The `github.com` hosted repository should have release items following `github`'s process.
//! This can be done by tagging a commit and then manually building a release where you
//! attach/upload `YourWorkflow.alfredworkflow` to the release page.
//!
//! The tag should follow all the [semantic versioning] rules.
//! The only exception to those rules is that you can prepend your
//! semantic version tag with ASCII letter `v`: `v0.3.1` or `0.3.1`
//!
//! You can easily create `YourWorkflow.alfredworkflow` file by using the [export feature] of
//! Alfred in its preferences window.
//!
//! ## Note to workflow authors
//! - Depending on network quality, checking if an update is available may take a long time.
//! This module may spawn a worker thread so that the check does not block the main flow of your plugin.
//! However given the limitations of Alfred's plugin architecture, the worker thread cannot outlive
//! your plugin's executable. This means that you either have to wait/block for the worker thread,
//! or if it is taking longer than a desirable time, you will have to abandon it.
//! See the example for more details.
//! - Workflow authors should make sure that _released_ workflow bundles have
//! their version set in [Alfred's preferences window]. However, this module provides
//! [`set_version()`] to set the version during runtime.
//!
//! [`Releaser`]: trait.Releaser.html
//! [`Updater`]: struct.Updater.html
//! [`update_ready()`]: struct.Updater.html#method.update_ready
//! [`try_update_ready()`]: struct.Updater.html#method.try_update_ready
//! [`download_latest()`]: struct.Updater.html#method.download_latest
//! [`Updater::gh()`]: struct.Updater.html#method.gh
//! [`Updater::new()`]: struct.Updater.html#method.new
//! [semantic versioning]: https://semver.org
//! [export feature]: https://www.alfredapp.com/help/workflows/advanced/sharing-workflows/
//! [Alfred's preferences window]: https://www.alfredapp.com/help/workflows/advanced/variables/
//! [`set_version()`]: struct.Updater.html#method.set_version
//! [`set_interval()`]: struct.Updater.html#method.set_interval
//!
//! # Example
//!
//! Create an updater for a workflow hosted on `github.com/spamwax/alfred-pinboard-rs`.
//! By default, it will check for new releases every 24 hours.
//! To change the interval, use [`set_interval()`] method.
//!
//! ```rust,no_run
//! # extern crate alfred;
//! # extern crate failure;
//! use alfred::{Item, ItemBuilder, Updater, json};
//!
//! # use std::io;
//! # use failure::Error;
//! # fn produce_items_for_user_to_see<'a>() -> Vec<Item<'a>> {
//! #     Vec::new()
//! # }
//! # fn do_some_other_stuff() {}
//! // Our workflow's main 'runner' function
//! fn run<'a>() -> Result<Vec<Item<'a>>, Error> {
//!     let updater = Updater::gh("spamwax/alfred-pinboard-rs")?;
//!
//!     // Start the process for getting latest release info
//!     updater.init().expect("cannot initialize updater");
//!
//!     // We'll do some other work that's related to our workflow:
//!     do_some_other_stuff();
//!     let mut items: Vec<Item> = produce_items_for_user_to_see();
//!
//!     // We can now check if update is ready using two methods:
//!     // 1- Block and wait until we receive results from worker thread
//!     //    It's a good practice to only wait for worker for a limited time so
//!     //    our workflow doesn't become unresponsive (not shown here)
//!     let update_status = updater.update_ready();
//!
//!     // 2- Or without blocking, check if the worker thread sent the results.
//!     //    If the worker thread is still busy, we'll get an `Err`
//!     let update_status = updater.try_update_ready();
//!
//!     if let Ok(is_ready) = update_status { // Comm. with worker was successful
//!         // Check for new update and add an item to 'items'
//!         if is_ready {
//!             let update_item = ItemBuilder::new(
//!                 "New version is available!"
//!             ).into_item();
//!             items.push(update_item);
//!         }
//!     } else {
//!         /* worker thread wasn't successful */
//!     }
//!     Ok(items)
//! }
//!
//! fn main() {
//!     // Fetch the items and show them.
//!     if let Ok(ref items) = run() {
//!         json::write_items(io::stdout(), items);
//!     }
//! }
//! ```
//!
//! An *issue* with above example can be when user is on a poor network or server is unresponsive.
//! In this case, the above snippet will try to call server every time workflow is invoked
//! by Alfred until the operation succeeds.

use chrono::prelude::*;
use env;
use failure::{err_msg, Error};
use reqwest;
use semver::Version;
use serde_json;
use std::cell::Cell;
use std::cell::RefCell;
use std::env as StdEnv;
use std::fs::{create_dir_all, remove_file, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use time::Duration;
use url::Url;
use url_serde;

mod imp;
mod releaser;

#[cfg(test)]
mod tests;

/// Default update interval duration (24 hrs)
pub const UPDATE_INTERVAL: i64 = 24 * 60 * 60;

pub use self::releaser::GithubReleaser;
pub use self::releaser::Releaser;

// TODO: Update Releaser trait so we don't need two methods (lastest_version and downloadable_url)
//     Only one method (latest_release?) should return both version and a download url.

/// Struct to check for & download the latest release of workflow from a remote server.
pub struct Updater<T>
where
    T: Releaser,
{
    state: imp::UpdaterState,
    releaser: RefCell<T>,
}

impl Updater<GithubReleaser> {
    /// Create an `Updater` object that will interface with a `github` repository.
    ///
    /// The `repo_name` should be in `user_name/repository_name` form. See the
    /// [module level documentation](./index.html) for full example and description.
    ///
    /// ```rust
    /// # extern crate alfred;
    /// use alfred::Updater;
    /// # use std::env;
    /// # fn main() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let updater = Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot initiate Updater");
    /// # }
    /// ```
    ///
    /// This only creates an `Updater` without performing any network operations.
    /// To check availability of a new release, launch and check for updates by
    /// using [`init()`] and [`update_ready()`] or [`try_update_ready()`] methods.
    ///
    /// To download an available release use [`download_latest()`] method afterwards.
    ///
    /// # Errors
    /// Error will happen during calling this method if:
    /// - `Updater` state cannot be read/written during instantiation, or
    /// - The workflow version cannot be parsed as semantic version compatible identifier.
    ///
    /// [`init()`]: struct.Updater.html#method.init
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    /// [`try_update_ready()`]: struct.Updater.html#method.try_update_ready
    /// [`download_latest()`]: struct.Updater.html#method.download_latest
    pub fn gh<S>(repo_name: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let releaser = GithubReleaser::new(repo_name);

        Self::load_or_new(releaser)
    }
}

impl<T> Updater<T>
where
    T: Releaser + Send + 'static,
{
    /// Create an `Updater` object that will interface with a remote repository for updating operations.
    ///
    /// `repo_name` is an arbitrary tag/identifier associated with the remote repository.
    ///
    /// How the `Updater` interacts with the remote server should be implemented using the [`Releaser`]
    /// trait. This crate provides a default implementation for interacting with
    /// `github.com` repositories, see [`gh()`] and [`GithubReleaser`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate alfred;
    /// # extern crate semver;
    /// # extern crate failure;
    /// # extern crate url;
    ///
    /// use url::Url;
    /// use semver::Version;
    ///
    /// use alfred::Updater;
    /// use alfred::updater::Releaser;
    /// # use std::env;
    /// # use failure::Error;
    /// # fn main() {
    ///
    /// #[derive(Clone)]
    /// struct MyPrivateHost {/* inner */};
    ///
    /// // You need to actually implement the trait, following is just a mock.
    /// impl Releaser for MyPrivateHost {
    ///     type SemVersion = Version;
    ///     type DownloadLink = Url;
    ///
    ///     fn new<S: Into<String>>(project_id: S) -> Self {
    ///         MyPrivateHost {}
    ///     }
    ///
    ///     fn fetch_latest_release(&self) -> Result<(Version, Url), Error> {
    ///         let version = Version::from((1, 0, 12));
    ///         let url = Url::parse("https://ci.remote.cc/release/latest")?;
    ///         Ok((version, url))
    ///     }
    /// }
    ///
    /// let updater: Updater<MyPrivateHost> =
    ///     Updater::new("my_hidden_proj").expect("cannot initiate Updater");
    /// # }
    /// ```
    ///
    /// This only creates an `Updater` without performing any network operations.
    /// To check availability of a new release, launch and check for updates by
    /// using [`init()`] and [`update_ready()`] or [`try_update_ready()`] methods.
    ///
    /// To check availability of a new release use [`update_ready()`] method.
    ///
    /// To download an available release use [`download_latest()`] method afterwards.
    ///
    /// # Errors
    /// Error will happen during calling this method if:
    /// - `Updater` state cannot be read/written during instantiation, or
    /// - The workflow version cannot be parsed as a semantic version compatible identifier.
    ///
    /// [`init()`]: struct.Updater.html#method.init
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    /// [`try_update_ready()`]: struct.Updater.html#method.try_update_ready
    /// [`download_latest()`]: struct.Updater.html#method.download_latest
    /// [`Releaser`]: trait.Releaser.html
    /// [`GithubReleaser`]: struct.GithubReleaser.html
    /// [`gh()`]: struct.Updater.html#method.gh
    pub fn new<S>(repo_name: S) -> Result<Updater<T>, Error>
    where
        S: Into<String>,
    {
        let releaser = Releaser::new(repo_name);
        Self::load_or_new(releaser)
    }

    /// Initializes `Updater` to fetch latest release information.
    ///
    /// - If it has been more than [`UPDATE_INTERVAL`] seconds (see [`set_interval()`]) since last check,
    /// the method will spawn a worker thread.
    /// In the background, the spawned thread will attempt to make a network call to fetch metadata of releases
    /// *only if* `UPDATE_INTERVAL` seconds has passed since the last network call.
    ///
    /// - All calls, which happen before the `UPDATE_INTERVAL` seconds, will initialize the `Updater`
    /// by using a local cache to report metadata about a release.
    ///
    /// For `Updater`s talking to `github.com`, the worker thread will only fetch a small
    /// metadata information to extract the version of the latest release.
    ///
    /// To check on status of worker thread and to get latest release status, use either of
    /// [`update_ready()`] or [`try_update_ready()`] methods.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate alfred;
    /// # extern crate failure;
    /// # use failure::Error;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # fn do_some_other_stuff() {}
    /// # fn test_async() -> Result<(), Error> {
    /// let updater = Updater::gh("spamwax/alfred-pinboard-rs")?;
    ///
    /// let rx = updater.init().expect("Error in starting updater.");
    ///
    /// // We'll do some other work that's related to our workflow while waiting
    /// do_some_other_stuff();
    ///
    /// // We can now check if update is ready using two methods:
    /// // 1- Block and wait until we receive results or errors
    /// let update_status = updater.update_ready();
    ///
    /// // 2- Or without blocking, check if the worker thread sent the results.
    /// //    If the worker thread is still busy, we'll get an `Err`
    /// let update_status = updater.try_update_ready();
    ///
    /// if let Ok(is_ready) = update_status {
    ///     // Everything went ok:
    ///     // No error happened during operation of worker thread
    ///     // and we received release info
    ///     if is_ready {
    ///         // there is an update available.
    ///     }
    /// } else {
    ///     /* either the worker thread wasn't successful or we couldn't get its results */
    /// }
    /// # Ok(())
    /// # }
    /// # fn main() {
    /// # test_async();
    /// # }
    /// ```
    ///
    /// # Errors
    /// Followings can cause the method return an error:
    /// - A worker thread cannot be spawned
    /// - Alfred environment variable error
    /// - File IO error
    ///
    /// [`set_interval()`]: struct.Updater.html#method.set_interval
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    /// [`try_update_ready()`]: struct.Updater.html#method.try_update_ready
    /// [`UPDATE_INTERVAL`]: constant.UPDATE_INTERVAL.html
    pub fn init(&self) -> Result<(), Error> {
        use self::imp::LATEST_UPDATE_INFO_CACHE_FN_ASYNC;
        use std::sync::mpsc;

        // file for status of last update check
        let p = Self::build_data_fn()?.with_file_name(LATEST_UPDATE_INFO_CACHE_FN_ASYNC);

        let (tx, rx) = mpsc::channel();

        if self.last_check().is_none() {
            self.set_last_check(Utc::now());
            self.save()?;
            // This send is always successful
            tx.send(Ok(None)).unwrap();
        } else if self.due_to_check() {
            // it's time to talk to remote server
            self.start_releaser_worker(tx, p)?;
        } else {
            let status = Self::read_last_check_status(&p)
                .map(|last_check| {
                    last_check.and_then(|info| {
                        if self.current_version() < info.version() {
                            Some(info)
                        } else {
                            None
                        }
                    })
                })
                .or(Ok(None));
            tx.send(status).unwrap();
        }
        *self.state.borrow_worker_mut() = Some(imp::MPSCState::new(rx));
        Ok(())
    }

    /// Checks if a new update is available by waiting for the background thread to finish
    /// fetching release info (blocking).
    ///
    /// In practice, this method will block if it has been more than [`UPDATE_INTERVAL`] seconds
    /// since last check. In any other instance the updater will return the update status
    /// that was cached since last check.
    ///
    /// This method will wait for worker thread (spawned by calling [`init()`]) to deliver release
    /// information from remote server.
    /// Upon successfull retreival, this method will compare release information to the current
    /// vertion of the workflow. The remote repository should tag each release according to semantic
    /// version scheme for this to work.
    ///
    /// You should use this method after calling `init()`, preferably after your workflow is done with other tasks
    /// and now wants to get information about the latest release.
    ///
    /// # Note
    ///
    /// - Since this method may block the current thread until a response is received from remote server,
    /// workflow authors should consider scenarios where network connection is poor and the block can
    /// take a long time (>1 second), and devise their workflow around it. An alternative to
    /// this method is the non-blocking [`try_update_ready()`].
    /// - The *very first* call to this method will always return false since it is assumed that
    /// user has just downloaded and installed the workflow.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate alfred;
    /// # extern crate failure;
    /// use alfred::Updater;
    ///
    /// # use failure::Error;
    /// # use std::io;
    /// # fn main() {
    /// let updater =
    ///     Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot initiate Updater");
    /// updater.init().expect("cannot start the worker thread");
    ///
    /// // Perform other workflow related tasks...
    ///
    /// assert_eq!(true, updater.update_ready().expect("cannot get update information"));
    ///
    /// # }
    /// ```
    ///
    /// # Errors
    /// Error will be returned :
    /// - If worker thread has been interrupted
    /// - If [`init()`] method has not been called successfully before this method
    /// - If worker could not communicate with server
    /// - If any file error or Alferd environment variable error happens
    ///
    /// [`init()`]: struct.Updater.html#method.init
    /// [`try_update_ready()`]: struct.Updater.html#method.try_update_ready
    /// [`UPDATE_INTERVAL`]: constant.UPDATE_INTERVAL.html
    pub fn update_ready(&self) -> Result<bool, Error> {
        if self.state.borrow_worker().is_none() {
            self.update_ready_sync()
        } else {
            self.update_ready_async(false)
        }
    }

    /// Try to get release info from background worker and see if a new update is available (non-blocking).
    ///
    /// This method will attempt to receive release information from worker thread
    /// (spawned by calling [`init()`]). Upon successfull retreival, this method will compare
    /// release information to the current vertion of the workflow.
    /// The remote repository should tag each release according to semantic version scheme
    /// for this to work.
    ///
    /// If communication with worker thread is not successful or if the worker thread could not
    /// fetch release information, this method will return an error.
    ///
    /// You should use this method after calling `init()`, preferably after your workflow is done with other tasks
    /// and now wants to get information about the latest release.
    ///
    /// # Note
    ///
    /// - To wait for the worker thread to deliver its release information you can use the blocking
    /// [`update_ready()`] method.
    /// - The *very first* call to this method will always return false since it is assumed that
    /// user has just downloaded and installed the workflow.
    ///
    /// # Example
    ///
    /// ```no_run
    /// extern crate alfred;
    /// # extern crate failure;
    ///
    /// use alfred::Updater;
    ///
    /// # use failure::Error;
    /// # use std::io;
    ///
    /// # fn do_some_other_stuff() {}
    ///
    /// fn main() {
    /// let updater =
    ///     Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot initiate Updater");
    /// updater.init().expect("cannot start the worker thread");
    ///
    /// // Perform other workflow related tasks...
    /// do_some_other_stuff();
    ///
    /// assert_eq!(true, updater.try_update_ready().expect("cannot get update information"));
    ///
    /// // Execution of program will immediately follow to here since this method is non-blocking.
    ///
    /// }
    /// ```
    ///
    /// # Errors
    /// Error will be returned :
    /// - If worker thread is not ready to send information or it has been interrupted
    /// - If [`init()`] method has not been called successfully before this method
    /// - If worker could not communicate with server
    /// - If any file error or Alferd environment variable error happens
    ///
    /// [`init()`]: struct.Updater.html#method.init
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    pub fn try_update_ready(&self) -> Result<bool, Error> {
        if self.state.borrow_worker().is_none() {
            self.update_ready_sync()
        } else {
            self.update_ready_async(true)
        }
    }

    /// Set workflow's version to `version`.
    ///
    /// Content of `version` needs to follow semantic versioning.
    ///
    /// This method is provided so workflow authors can set the version from within the Rust code.
    ///
    /// For example, by reading cargo or git info during compile time and using this method to
    /// assign the version to workflow.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate alfred;
    /// # extern crate failure;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # use failure::Error;
    /// # fn ex_set_version() -> Result<(), Error> {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let mut updater = Updater::gh("spamwax/alfred-pinboard-rs")?;
    /// updater.set_version("0.23.3");
    /// # Ok(())
    /// # }
    ///
    /// # fn main() {
    /// #     ex_set_version();
    /// # }
    /// ```
    /// An alternative (recommended) way of setting version is through [Alfred's preferences window].
    ///
    /// [Alfred's preferences window]: https://www.alfredapp.com/help/workflows/advanced/variables/
    ///
    /// # Panics
    /// The method will panic if the passed value `version` cannot be parsed as a semantic version compatible string.
    pub fn set_version<S: AsRef<str>>(&mut self, version: S) {
        let v = Version::parse(version.as_ref())
            .expect("version should follow semantic version rules.");
        self.state.set_version(v);

        StdEnv::set_var("alfred_workflow_version", version.as_ref());
    }

    /// Set the interval between checks for a newer release (in seconds)
    ///
    /// [Default value][`UPDATE_INTERVAL`] is 86,400 seconds (24 hrs).
    ///
    /// # Example
    /// Set interval to be 7 days
    ///
    /// ```rust
    /// # extern crate alfred;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # fn main() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let mut updater =
    ///     Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot initiate Updater");
    /// updater.set_interval(7 * 24 * 60 * 60);
    /// # }
    /// ```
    /// [`UPDATE_INTERVAL`]: constant.UPDATE_INTERVAL.html
    pub fn set_interval(&mut self, tick: i64) {
        self.set_update_interval(tick);
    }

    /// Check if it is time to ask remote server for latest updates.
    ///
    /// It returns `true` if it has been more than [`UPDATE_INTERVAL`] seconds since we last
    /// checked with server (i.e. ran [`update_ready()`]), otherwise returns false.
    ///
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate alfred;
    /// # extern crate failure;
    /// # use alfred::Updater;
    /// # use failure::Error;
    /// # fn run() -> Result<(), Error> {
    /// let mut updater = Updater::gh("spamwax/alfred-pinboard-rs")?;
    ///
    /// // Assuming it is has been UPDATE_INTERVAL seconds since last time we ran the
    /// // `update_ready()` and there actually exists a new release:
    /// assert_eq!(true, updater.due_to_check());
    /// # Ok(())
    /// # }
    /// # fn main() {
    /// # run();
    /// # }
    /// ```
    ///
    /// [`UPDATE_INTERVAL`]: constant.UPDATE_INTERVAL.html
    pub fn due_to_check(&self) -> bool {
        self.last_check().map_or(true, |dt| {
            Utc::now().signed_duration_since(dt) > Duration::seconds(self.update_interval())
        })
    }

    /// Function to download and save the latest release into workflow's cache dir.
    ///
    /// If the download and save operations are both successful, it returns name of file in which the
    /// downloaded Alfred workflow bundle is saved.
    ///
    /// The downloaded workflow will be saved in dedicated cache folder of the workflow, and it
    /// will be always renamed to `latest_release_WORKFLOW-UID.alfredworkflow`
    ///
    /// To install the downloaded release, your workflow needs to somehow open the saved file.
    ///
    /// Within shell, it can be installed by issuing something like:
    /// ```bash
    /// open -b com.runningwithcrayons.Alfred-3 latest_release_WORKFLOW-UID.alfredworkflow
    /// ```
    ///
    /// Or you can add "Run script" object to your workflow and use environment variables set by
    /// Alfred to automatically open the downloaded release:
    /// ```bash
    /// open -b com.runningwithcrayons.Alfred-3 "$alfred_workflow_cache/latest_release_$alfred_workflow_uid.alfredworkflow"
    /// ```
    ///
    /// # Note
    ///
    /// The method may take longer than other Alfred-based actions to complete. Workflow authors using this crate
    /// should implement strategies to prevent unpleasant long blocks of user's typical work flow.
    ///
    /// One option to initiate the download and upgrade process is to invoke your executable with a
    /// different argument. The following snippet can be tied to a dedicated Alfred **Hotkey**
    /// or **Script Filter** so that it is only executed when user explicitly asks for it:
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate alfred;
    /// # extern crate failure;
    /// # use alfred::Updater;
    /// # use std::io;
    /// use alfred::{ItemBuilder, json};
    ///
    /// # fn main() {
    /// # let updater =
    /// #    Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot initiate Updater");
    /// # let cmd_line_download_flag = true;
    /// if cmd_line_download_flag && updater.update_ready().unwrap() {
    ///     match updater.download_latest() {
    ///         Ok(downloaded_fn) => {
    ///             json::write_items(io::stdout(), &[
    ///                 ItemBuilder::new("New version of workflow is available!")
    ///                              .subtitle("Click to upgrade!")
    ///                              .arg(downloaded_fn.to_str().unwrap())
    ///                              .variable("update_ready", "yes")
    ///                              .valid(true)
    ///                              .into_item()
    ///             ]);
    ///         },
    ///         Err(e) => {
    ///             // Show an error message to user or log it.
    ///         }
    ///     }
    /// }
    /// #    else {
    /// #    }
    /// # }
    /// ```
    ///
    /// For the above example to automatically work, you then need to connect the output of the script
    /// to an **Open File** action so that Alfred can install/upgrade the new version.
    ///
    /// As suggested in above example, you can add an Alfred variable to the item so that your workflow
    /// can use it for further processing.
    ///
    /// # Errors
    /// Downloading latest workflow can fail if network error, file error or Alfred environment variable
    /// errors happen, or if [`Releaser`] cannot produce a usable download url.
    ///
    /// [`Releaser`]: trait.Releaser.html
    pub fn download_latest(&self) -> Result<PathBuf, Error> {
        // let url = self.releaser.borrow().downloadable_url()?;
        let url = self.state
            .download_url()
            .ok_or(err_msg("no release info avail yet"))?;
        let client = reqwest::Client::new();

        client
            .get(url)
            .send()?
            .error_for_status()
            .map_err(|e| e.into())
            .and_then(|mut resp| {
                // Get workflow's dedicated cache folder & build a filename
                let latest_release_downloaded_fn = env::workflow_cache()
                    .ok_or_else(|| err_msg("missing env variable for cache dir"))
                    .and_then(|mut cache_dir| {
                        env::workflow_uid()
                            .ok_or_else(|| err_msg("missing env variable for uid"))
                            .and_then(|ref uid| {
                                cache_dir
                                    .push(["latest_release_", uid, ".alfredworkflow"].concat());
                                Ok(cache_dir)
                            })
                    })?;
                // Save the file
                File::create(&latest_release_downloaded_fn)
                    .map_err(|e| e.into())
                    .and_then(|fp| {
                        let mut buf_writer = BufWriter::with_capacity(0x10_0000, fp);
                        resp.copy_to(&mut buf_writer)?;
                        Ok(())
                    })
                    .or_else(|e: Error| {
                        let _ = remove_file(&latest_release_downloaded_fn);
                        Err(e)
                    })?;
                Ok(latest_release_downloaded_fn)
            })
    }
}
