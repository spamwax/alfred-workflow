//! Helper to store persistent or temporary data to disk.
//!
//! This module provides methods to store workflow related data
//! (such as settings, configurations, ...) to disk. Additionally using the non-method functions
//! workflow authors can save/load data to workflow's cache directory.
//!
//! To store/retrieve workflow related data, use [`set()`] and [`get()`] method after [`load()`]ing.
//! Example of such data can be authentication info related to workflow or how many items
//! you should show to user in Alfred's main window.
//!
//! To save/load temporary data, use [`save_to_file()`] and [`load_from_file()`] functions.
//! Example of such data are cached list of items related to workflow or a downloaded file to be used later.
//!
//! # Example
//! ```rust,no_run
//! # extern crate alfred_rs;
//! extern crate chrono;
//! # use chrono::prelude::*;
//! use alfred_rs::data::Data;
//!
//! // Load the workflow data (or create a new one)
//! let mut workflow_data = Data::load("settings.json").unwrap();
//!
//! // Set *and* save key/value `user_id: 0xFF` pair
//! workflow_data.set("user_id", &0xFF);
//!
//! // We can set/save different data types.
//! // For example, set and save timestamp of last use of workflow:
//! workflow_data.set("last_use_date", &Utc::now());
//!
//!
//! // Later on, you can retreive the values:
//! let last_use: DateTime<Utc> =
//!     workflow_data.get("last_use_date").expect("timestamp was not set");
//!
//! // Additioanlly, you can save temporary data to workflow's cache folder:
//! Data::save_to_file("all_my_tweets.cache", &vec!["chirp1", "chirp2"]).unwrap();
//! ```
//!
//! See `Data`'s [documentation] for more examples.
//!
//! [`load()`]: struct.Data.html#method.load
//! [`set()`]: struct.Data.html#method.set
//! [`get()`]: struct.Data.html#method.get
//! [`save_to_file()`]: struct.Data.html#method.save_to_file
//! [`load_from_file()`]: struct.Data.html#method.load_from_file
//! [documentation]: struct.Data.html
use super::*;

use serde::Deserialize;
use serde::Serialize;
use serde_json::{from_value, to_value, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Workflow data that will be persisted to disk
#[derive(Debug)]
pub struct Data {
    inner: HashMap<String, Value>,
    file_name: PathBuf,
}

impl Data {
    /// Loads the workflow data or creates a new one.
    ///
    /// Reads the data stored in `p` file.
    /// Only file name section of `p` is used as data will be always saved
    /// in workflow's default data dir.
    /// If the file is missing or corrupt a new (empty) Data instance will be returned.
    ///
    /// # Errors:
    /// This method can fail if any disk/IO error happens.
    pub fn load<P: AsRef<Path>>(p: P) -> Result<Self, Error> {
        if p.as_ref().as_os_str().is_empty() {
            bail!("File name to load data from cannot be empty");
        }

        // Only use the file name section of input parameter. We will always save to Workflow's
        // data dir
        let filename = p
            .as_ref()
            .file_name()
            .ok_or_else(|| err_msg("invalid file name"))?;
        let wf_data_path = env::workflow_data().ok_or_else(|| {
            err_msg("missing env variable for data dir. forgot to set workflow bundle id?")
        })?;

        let wf_data_fn = wf_data_path.join(filename);

        let inner = Self::read_data_from_disk(&wf_data_fn)
            .or_else(|_| -> Result<_, Error> { Ok(HashMap::new()) })?;
        Ok(Data {
            inner,
            file_name: wf_data_fn,
        })
    }

    /// Set the value of key `k` to `v` and persist it to disk
    ///
    /// `k` is a type that implements `Into<String>`. `v` can be any type as long as it
    /// implements `Serialize`.
    ///
    /// This method overwrites values of any existing keys, otherwise adds the key/value pair
    /// to the workflow's standard data file
    ///
    /// # Example
    /// ```rust,no_run
    /// # extern crate alfred_rs;
    /// # extern crate chrono;
    /// # use chrono::prelude::*;
    /// use alfred_rs::data::Data;
    ///
    /// let mut workflow_data = Data::load("settings.json").unwrap();
    ///
    /// workflow_data.set("user_id", &0xFF);
    /// workflow_data.set("last_log_date", &Utc::now());
    /// ```
    /// # Errors:
    /// If `v` cannot be serialized or there are file IO issues an error is returned.
    pub fn set<K, V>(&mut self, k: K, v: &V) -> Result<(), Error>
    where
        K: Into<String>,
        V: Serialize,
    {
        let v = to_value(v)?;
        self.inner.insert(k.into(), v);
        Self::write_data_to_disk(&self.file_name, &self.inner)
    }

    /// Get (possible) value of key `k` from workflow's data
    ///
    /// If key `k` has not been set before `None` will be returned.
    ///
    /// Since the data can be of arbitrary type, you should annotate the type you are expecting
    /// to get back from data file.
    /// If the stored value cannot be deserialized back to the desired type `None` is returned.
    ///
    /// # Example
    /// ```rust,no_run
    /// # extern crate alfred_rs;
    /// # extern crate chrono;
    /// # use chrono::prelude::*;
    /// use alfred_rs::data::Data;
    ///
    /// let wf_data = Data::load("settings.json").unwrap();
    ///
    /// let id: i32 = wf_data.get("user_id").expect("user id was not set");
    /// let last_log: DateTime<Utc> = wf_data.get("last_log_date").expect("log date was not set");
    /// ```
    pub fn get<K, V>(&self, k: K) -> Option<V>
    where
        K: AsRef<str>,
        V: for<'d> Deserialize<'d>,
    {
        self.inner
            .get(k.as_ref())
            .and_then(|v| from_value(v.clone()).ok())
    }

    /// Clear all key-value pairs. Does not affect data on disk.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Function to save (temporary) `data` to file named `p` in workflow's cache dir
    ///
    /// This function is provided so that workflow authors can temporarily save information
    /// to workflow's cache dir. The saved data is considered to be irrelevant to workflow's
    /// actual data (for which you should use [`set`] and [`get`])
    ///
    /// # Example
    /// ```rust,no_run
    /// # extern crate alfred_rs;
    /// use alfred_rs::data::Data;
    ///
    /// Data::save_to_file("cached_tags.dat", &vec!["rust", "alfred"]).unwrap();
    /// ```
    /// ## Note
    /// Only the [`file_name`] portion of `p` will be used to name the file that'll be stored in
    /// workflow's cache directory.
    /// # Errors
    /// File IO related issues as well as serializing problems will cause an error to be returned.
    ///
    /// [`set`]: struct.Data.html#method.set
    /// [`get`]: struct.Data.html#method.get
    /// [`file_name`]: https://doc.rust-lang.org/std/path/struct.Path.html#method.file_name
    pub fn save_to_file<P, V>(p: P, data: &V) -> Result<(), Error>
    where
        P: AsRef<Path>,
        V: Serialize,
    {
        let filename = p
            .as_ref()
            .file_name()
            .ok_or_else(|| err_msg("invalid file name"))?;
        let p = env::workflow_cache()
            .map(|wfc| wfc.join(filename))
            .ok_or_else(|| {
                err_msg("missing env variable for cache dir. forgot to set workflow bundle id?")
            })?;
        debug!("saving to: {}", p.to_str().expect(""));
        Self::write_data_to_disk(p, data)
    }

    fn write_data_to_disk<P, V>(p: P, data: &V) -> Result<(), Error>
    where
        P: AsRef<Path> + std::fmt::Debug,
        V: Serialize,
    {
        use tempfile::Builder;
        let wfc = env::workflow_cache().ok_or_else(|| {
            err_msg("missing env variable for cache dir. forgot to set workflow bundle id?")
        })?;
        let named_tempfile = Builder::new()
            .prefix("alfred_rs_temp")
            .suffix(".json")
            .rand_bytes(5)
            .tempfile_in(wfc)?;

        let fn_temp = named_tempfile.as_ref();
        File::create(&fn_temp).and_then(|fp| {
            let buf_writer = BufWriter::with_capacity(0x1000, fp);
            serde_json::to_writer(buf_writer, data)?;
            Ok(())
        })?;

        // Rename over to main file name
        use std::fs;
        fs::rename(fn_temp, p)?;
        Ok(())
    }

    /// Function to load some (temporary) data from file named `p` in workflow's cache dir
    ///
    /// This function is provided so that workflow authors can retrieve temporarily information
    /// saved to workflow's cache dir. The saved data is considered to be irrelevant to workflow's
    /// actual data (for which you should use [`set`] and [`get`])
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate alfred_rs;
    /// use alfred_rs::data::Data;
    ///
    /// let cached_tags: Vec<String> = Data::load_from_file("cached_tags.dat").unwrap();
    /// ```
    ///
    /// ## Note
    /// Only the [`file_name`] portion of `p` will be used to name the file, which will then be
    /// looked up in workflow's cache directory.
    ///
    /// [`set`]: struct.Data.html#method.set
    /// [`get`]: struct.Data.html#method.get
    /// [`file_name`]: https://doc.rust-lang.org/std/path/struct.Path.html#method.file_name
    pub fn load_from_file<P, V>(p: P) -> Option<V>
    where
        P: AsRef<Path>,
        V: for<'d> Deserialize<'d>,
    {
        let p = env::workflow_cache()
            .and_then(|wfc| p.as_ref().file_name().map(|name| wfc.join(name)))?;
        debug!("loading from: {}", p.to_str().expect(""));
        Self::read_data_from_disk(&p).ok()
    }

    fn read_data_from_disk<V>(p: &Path) -> Result<V, Error>
    where
        V: for<'d> Deserialize<'d>,
    {
        File::open(p).map_err(|e| e.into()).and_then(|fp| {
            let buf_reader = BufReader::with_capacity(0x1000, fp);
            let d: V = serde_json::from_reader(buf_reader)?;
            Ok(d)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;
    use std::env as StdEnv;
    use std::ffi::OsStr;
    use std::fs::remove_file;
    use std::{thread, time};
    use tempfile::Builder;

    #[test]
    fn it_sets_gets_data() {
        #[derive(Serialize, Deserialize)]
        struct User {
            name: String,
            age: usize,
        }

        setup_workflow_env_vars(true);

        let user = User {
            name: "Hamid".to_string(),
            age: 42,
        };

        {
            let mut wf_data: Data = Data::load("settings_test.json").unwrap();
            wf_data.set("key1", &8).unwrap();
            wf_data.set("key2", &user).unwrap();
            wf_data.set("date", &Utc::now()).unwrap();
            println!("{:?}", wf_data);
        }

        {
            let wf_data = Data::load("settings_test.json").unwrap();

            assert_eq!(3, wf_data.inner.len());
            let user: User = wf_data.get("key2").unwrap();
            assert_eq!(42, user.age);

            let x: i8 = wf_data.get("key1").unwrap();
            assert_eq!(8, x);
            let _last_log: DateTime<Utc> = wf_data.get("date").expect("log date was not set");
        }
    }

    #[test]
    fn it_saves_loads_from_file() {
        let wfc = setup_workflow_env_vars(true);
        let path = wfc.join("_test_saves_loads_from_file");
        let _ = remove_file(&path);

        let now = Utc::now();
        Data::save_to_file(&path, &now).expect("couldn't write to file");
        let what_now: DateTime<Utc> =
            Data::load_from_file(path).expect("couldn't get value from test file");
        assert_eq!(now, what_now);
    }

    #[test]
    fn it_overwrites_cached_data_file() {
        let wfc = setup_workflow_env_vars(true);
        let path = wfc.join("_test_it_overwrites_cached_data_file");
        let _ = remove_file(&path);

        let ten_millis = time::Duration::from_millis(10);

        let now1 = Utc::now();
        Data::save_to_file(&path, &now1).expect("couldn't write to file");

        thread::sleep(ten_millis);

        let now2 = Utc::now();
        Data::save_to_file(&path, &now2).expect("couldn't write to file");

        let what_now: DateTime<Utc> =
            Data::load_from_file(path).expect("couldn't get value from test file");
        assert_eq!(now2, what_now);
    }

    pub(super) fn setup_workflow_env_vars(secure_temp_dir: bool) -> PathBuf {
        // Mimic Alfred's environment variables
        let path = if secure_temp_dir {
            Builder::new()
                .prefix("alfred_workflow_test")
                .rand_bytes(5)
                .tempdir()
                .unwrap()
                .into_path()
        } else {
            StdEnv::temp_dir()
        };
        {
            let v: &OsStr = path.as_ref();
            StdEnv::set_var("alfred_workflow_data", v);
            StdEnv::set_var("alfred_workflow_cache", v);
            StdEnv::set_var("alfred_workflow_uid", "workflow.B0AC54EC-601C");
            StdEnv::set_var(
                "alfred_workflow_name",
                "YouForgotTo/フ:Name好YouráOwnسWork}flowッ",
            );
            StdEnv::set_var("alfred_workflow_bundleid", "MY_BUNDLE_ID");
            StdEnv::set_var("alfred_workflow_version", "0.10.5");
        }
        path
    }
}
