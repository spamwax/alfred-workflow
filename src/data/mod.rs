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
//! let mut workflow_data = Data::load().unwrap();
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
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;

/// Workflow data that will be persisted to disk
#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    inner: HashMap<String, Value>,
}

impl Data {
    /// Loads the workflow data or creates a new one.
    ///
    /// Reads the content of workflow's standard data file `WORKFLOW_UID-persistent-data.json`.
    /// If the file is missing or corrupt a new (empty) data will be produced.
    ///
    /// # Errors:
    /// This method can fail if any disk/IO error happens.
    pub fn load() -> Result<Self, Error> {
        Self::read_data_from_disk().or_else(|_| {
            Ok(Data {
                inner: HashMap::new(),
            })
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
    /// let mut workflow_data = Data::load().unwrap();
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
        self.write_data_to_disk()
    }

    /// Get (possible) value of key `k` from workflow's data file
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
    /// let wf_data = Data::load().unwrap();
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

    /// Function to save (temporary) `data` to file named `p` in workflow's cache dir
    ///
    /// This function is provided so that workflow authors can temporarily save information
    /// to workflow's cache dir. The saved data is considered to be irrelevant to workflow's
    /// actual data (for which you should use [`set`] and [`get`])
    ///
    /// # Example
    /// ```rust,no_run
    /// # extern crate alfred_rs;
    /// # extern crate chrono;
    /// # use chrono::prelude::*;
    /// use alfred_rs::data::Data;
    ///
    /// Data::save_to_file("cached_tags.dat", &vec!["rust", "alfred"]).unwrap();
    /// ```
    /// ## Note
    /// Only the [`file_name`] portion of `p` will be used to name the file in
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
        File::create(p).map_err(|e| e.into()).and_then(|fp| {
            let buf_writer = BufWriter::with_capacity(0x1000, fp);
            serde_json::to_writer(buf_writer, data)?;
            Ok(())
        })
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
    /// # extern crate chrono;
    /// # use chrono::prelude::*;
    /// use alfred_rs::data::Data;
    ///
    /// let cached_tags: Vec<String> = Data::load_from_file("cached_tags.dat").unwrap();
    /// ```
    ///
    /// ## Note
    /// Only the [`file_name`] portion of `p` will be used to name the file in
    /// workflow's cache directory.
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
        File::open(p)
            .and_then(|fp| {
                let mut buf_reader = BufReader::with_capacity(0x1000, fp);
                let mut content = String::with_capacity(0x1000);
                buf_reader.read_to_string(&mut content)?;
                let d: V = serde_json::from_str(&content)?;
                Ok(d)
            })
            .ok()
    }

    fn read_data_from_disk() -> Result<Self, Error> {
        env::workflow_data()
            .ok_or_else(|| {
                err_msg("missing env variable for data dir. forgot to set workflow bundle id?")
            })
            .and_then(|wf_data_path| {
                let workflow_name = env::workflow_uid()
                    .map(|ref uid| [uid, "-persistent-data.json"].concat())
                    .unwrap_or_else(|| "unnamed_workflow".to_string());
                let wf_data_fn = wf_data_path.join(workflow_name);
                File::open(wf_data_fn).map_err(|e| e.into()).and_then(|fp| {
                    let mut buf_reader = BufReader::with_capacity(0x1000, fp);
                    let mut content = String::with_capacity(0x1000);
                    buf_reader.read_to_string(&mut content)?;
                    let d: Self = serde_json::from_str(&content)?;
                    Ok(d)
                })
            })
    }

    fn write_data_to_disk(&self) -> Result<(), Error> {
        env::workflow_data()
            .ok_or_else(|| {
                err_msg("missing env variable for data dir. forgot to set workflow bundle id?")
            })
            .and_then(|wf_data_path| {
                // Write to a temp file first
                let wf_data_fn_temp = wf_data_path.join("temp_persistent-data.json");
                File::create(&wf_data_fn_temp).and_then(|fp| {
                    let buf_writer = BufWriter::with_capacity(0x1000, fp);
                    serde_json::to_writer(buf_writer, &self)?;
                    Ok(())
                })?;

                // Rename over to main file name
                let workflow_name = env::workflow_uid()
                    .map(|ref uid| [uid, "-persistent-data.json"].concat())
                    .unwrap_or_else(|| "unnamed_workflow".to_string());
                let wf_data_fn = wf_data_path.join(workflow_name);
                use std::fs;
                fs::rename(wf_data_fn_temp, wf_data_fn)?;
                Ok(())
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
    use std::path::PathBuf;
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
            let mut wf_data: Data = Data::load().unwrap();
            println!("{:?}", wf_data);
            wf_data.set("key1", &8).unwrap();
            wf_data.set("key2", &user).unwrap();
            wf_data.set("date", &Utc::now()).unwrap();
            println!("{:?}", wf_data);
        }

        {
            let wf_data = Data::load().unwrap();

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
