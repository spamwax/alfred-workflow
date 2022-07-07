use super::*;
use crate::Updater;
use std::cell::Cell;
use std::cell::Ref;
use std::cell::RefMut;
use std::path::Path;
use std::sync::mpsc;

pub(super) const LATEST_UPDATE_INFO_CACHE_FN_ASYNC: &str = "last_check_status_async.json";

// Payload that the worker thread will send back
type ReleasePayloadResult = Result<Option<UpdateInfo>>;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct UpdaterState {
    pub(super) last_check: Cell<Option<DateTime<Utc>>>,

    current_version: Version,

    avail_release: RefCell<Option<UpdateInfo>>,

    #[serde(skip, default = "default_interval")]
    update_interval: i64,

    #[serde(skip)]
    worker_state: RefCell<Option<MPSCState>>,
}

impl UpdaterState {
    pub(super) fn current_version(&self) -> &Version {
        &self.current_version
    }

    pub(super) fn set_version(&mut self, v: Version) {
        self.current_version = v;
    }

    pub(super) fn latest_avail_version(&self) -> Option<Version> {
        self.avail_release
            .borrow()
            .as_ref()
            .map(|ui| ui.version().clone())
    }

    pub(super) fn borrow_worker(&self) -> Ref<'_, Option<MPSCState>> {
        self.worker_state.borrow()
    }

    pub(super) fn borrow_worker_mut(&self) -> RefMut<'_, Option<MPSCState>> {
        self.worker_state.borrow_mut()
    }

    pub(super) fn download_url(&self) -> Option<Url> {
        self.avail_release
            .borrow()
            .as_ref()
            .map(|info| info.downloadable_url.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) struct UpdateInfo {
    // Latest version available from github or releaser
    pub version: Version,

    pub fetched_at: Option<DateTime<Utc>>,

    // Link to use to download the above version
    #[serde(with = "url_serde")]
    pub downloadable_url: Url,
}

impl UpdateInfo {
    pub fn new(v: Version, url: Url) -> Self {
        UpdateInfo {
            version: v,
            fetched_at: None,
            downloadable_url: url,
        }
    }

    pub(super) fn version(&self) -> &Version {
        &self.version
    }

    pub(super) fn fetched_at(&self) -> Option<&DateTime<Utc>> {
        self.fetched_at.as_ref()
    }

    pub(super) fn set_fetched_at(&mut self, date_time: DateTime<Utc>) {
        self.fetched_at = Some(date_time);
    }
}

#[derive(Debug)]
pub(super) struct MPSCState {
    // First successful call on rx.recv() will cache the results into this field
    recvd_payload: RefCell<Option<ReleasePayloadResult>>,
    // Receiver end of communication channel with worker thread
    rx: RefCell<Option<Receiver<ReleasePayloadResult>>>,
}

impl MPSCState {
    pub(super) fn new(rx: mpsc::Receiver<ReleasePayloadResult>) -> Self {
        MPSCState {
            recvd_payload: RefCell::new(None),
            rx: RefCell::new(Some(rx)),
        }
    }
}

impl<T> Updater<T>
where
    T: Releaser + Send + 'static,
{
    pub(super) fn load_or_new(r: T) -> Result<Self> {
        let _ = env_logger::try_init();
        if let Ok(mut saved_state) = Self::load() {
            // Use the version that workflow reports through environment variable
            // This version takes priortiy over what we may have saved last time.
            let env_ver = env::workflow_version().and_then(|v| Version::parse(&v).ok());
            if let Some(v) = env_ver {
                saved_state.current_version = v;
            }
            Ok(Updater {
                state: saved_state,
                releaser: RefCell::new(r),
            })
        } else {
            let current_version = env::workflow_version()
                .map_or_else(|| Ok(Version::from((0, 0, 0))), |v| Version::parse(&v))?;
            let state = UpdaterState {
                current_version,
                last_check: Cell::new(None),
                avail_release: RefCell::new(None),
                worker_state: RefCell::new(None),
                update_interval: UPDATE_INTERVAL,
            };
            let updater = Updater {
                state,
                releaser: RefCell::new(r),
            };
            updater.save()?;
            Ok(updater)
        }
    }

    pub(super) fn last_check(&self) -> Option<DateTime<Utc>> {
        self.state.last_check.get()
    }

    pub(super) fn set_last_check(&self, t: DateTime<Utc>) {
        self.state.last_check.set(Some(t));
    }

    pub(super) fn update_interval(&self) -> i64 {
        self.state.update_interval
    }

    pub(super) fn set_update_interval(&mut self, t: i64) {
        self.state.update_interval = t;
    }

    fn load() -> Result<UpdaterState> {
        let data_file_path = Self::build_data_fn()?;
        crate::Data::load_from_file(data_file_path)
            .ok_or_else(|| anyhow!("cannot load cached state of updater"))
    }

    // Save updater's state
    pub(super) fn save(&self) -> Result<()> {
        let data_file_path = Self::build_data_fn()?;
        crate::Data::save_to_file(&data_file_path, &self.state).map_err(|e| {
            let _ = remove_file(data_file_path);
            e
        })
    }

    pub(super) fn start_releaser_worker(
        &self,
        tx: mpsc::Sender<ReleasePayloadResult>,
        p: PathBuf,
    ) -> Result<()> {
        use std::thread;

        let releaser = (*self.releaser.borrow()).clone();

        thread::Builder::new().spawn(move || {
            debug!("other thread: starting in updater thread");
            let talk_to_mother = || -> Result<()> {
                let (v, url) = releaser.latest_release()?;
                let mut info = UpdateInfo::new(v, url);
                info.set_fetched_at(Utc::now());
                let payload = Some(info);
                Self::write_last_check_status(&p, &payload)?;
                tx.send(Ok(payload))?;
                Ok(())
            };

            let outcome = talk_to_mother();
            debug!("other thread: finished checking releaser status");

            if let Err(error) = outcome {
                tx.send(Err(error))
                    .expect("could not send error from thread");
            }
        })?;
        Ok(())
    }

    // write version of latest avail. release (if any) to a cache file
    pub(super) fn write_last_check_status(
        p: &Path,
        updater_info: &Option<UpdateInfo>,
    ) -> Result<()> {
        crate::Data::save_to_file(p, updater_info).map_err(|e| {
            let _ = remove_file(p);
            e
        })
    }

    // read version of latest avail. release (if any) from a cache file
    pub(super) fn read_last_check_status(p: &Path) -> Result<Option<UpdateInfo>> {
        crate::Data::load_from_file(p).ok_or_else(|| anyhow!("no data in given path"))
    }

    pub(super) fn build_data_fn() -> Result<PathBuf> {
        let workflow_name = env::workflow_name()
            .unwrap_or_else(|| "YouForgotTo/フ:NameYourOwnWork}flowッ".to_string())
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();

        env::workflow_cache()
            .ok_or_else(|| {
                anyhow!("missing env variable for cache dir. forgot to set workflow bundle id?")
            })
            .and_then(|mut data_path| {
                env::workflow_uid()
                    .ok_or_else(|| anyhow!("missing env variable for uid"))
                    .map(|ref uid| {
                        let filename = [uid, "-", workflow_name.as_str(), "-updater.json"].concat();
                        data_path.push(filename);

                        data_path
                    })
            })
    }

    pub(super) fn update_ready_async(&self, try_flag: bool) -> Result<bool> {
        self.state
            .worker_state
            .borrow()
            .as_ref()
            .ok_or_else(|| anyhow!("you need to use init() method first."))
            .and_then(|mpsc| {
                if mpsc.recvd_payload.borrow().is_none() {
                    // No payload received yet, try to talk to worker thread
                    mpsc.rx
                        .borrow()
                        .as_ref()
                        .ok_or_else(|| anyhow!("you need to use init() correctly!"))
                        .and_then(|rx| {
                            let rr = if try_flag {
                                // don't block while trying to receive
                                rx.try_recv().map_err(|e| anyhow!(e.to_string()))
                            } else {
                                // block while waiting to receive
                                rx.recv().map_err(|e| anyhow!(e.to_string()))
                            };
                            rr.and_then(|msg| {
                                let msg_status = msg.map(|update_info| {
                                    // received good message, update cache for received payload
                                    *self.state.avail_release.borrow_mut() = update_info.clone();
                                    // update last_check if received info is newer than last_check
                                    update_info.as_ref().map(|ui| {
                                        ui.fetched_at().map(|fetched_time| {
                                            if self.last_check().is_none()
                                                || self.last_check().as_ref().unwrap()
                                                    < fetched_time
                                            {
                                                self.set_last_check(*fetched_time)
                                            }
                                        })
                                    });
                                    *mpsc.recvd_payload.borrow_mut() = Some(Ok(update_info));
                                });
                                // save state regardless of content of msg
                                self.save()?;
                                msg_status?;
                                Ok(())
                            })
                        })?;
                }
                Ok(())
            })?;
        Ok(self
            .state
            .avail_release
            .borrow()
            .as_ref()
            .map(|release| *self.current_version() < release.version)
            .unwrap_or(false))
    }

    #[allow(dead_code)]
    pub(super) fn _update_ready_async(&self) -> Result<bool> {
        let worker_state = self.state.worker_state.borrow();
        if worker_state.is_none() {
            panic!("you need to use init first")
        };

        let mpsc = worker_state.as_ref().expect("no worker_state");
        if mpsc.recvd_payload.borrow().is_none() {
            let rx_option = mpsc.rx.borrow();
            let rx = rx_option.as_ref().unwrap();
            let rr = rx.recv();
            if rr.is_ok() {
                let msg = rr.as_ref().unwrap();
                if msg.is_ok() {
                    let update_info = msg.as_ref().unwrap();
                    *self.state.avail_release.borrow_mut() = update_info.clone();
                    *mpsc.recvd_payload.borrow_mut() = Some(Ok(update_info.clone()));
                } else {
                    return Err(anyhow!(format!("{:?}", msg.as_ref().unwrap_err())));
                }
                self.save()?;
            } else {
                eprintln!("{:?}", rr);
                return Err(anyhow!(format!("{:?}", rr)));
            }
        }
        if let Some(ref updater_info) = *self.state.avail_release.borrow() {
            if *self.current_version() < updater_info.version {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    #[allow(dead_code)]
    pub(super) fn _update_ready_sync(&self) -> Result<bool> {
        // A None value for last_check indicates that workflow is being run for first time.
        // Thus we update last_check to now and just save the updater state without asking
        // Releaser to do a remote call/check for us since we assume that user just downloaded
        // the workflow.
        const LATEST_UPDATE_INFO_CACHE_FN: &str = "last_check_status.json";

        // file for status of last update check
        let p = Self::build_data_fn()?.with_file_name(LATEST_UPDATE_INFO_CACHE_FN);

        // make a network call to see if a newer version is avail.
        // save the result of call to cache file.
        let ask_releaser_for_update = || -> Result<bool> {
            let (v, url) = self.releaser.borrow().latest_release()?;
            let update_avail = *self.current_version() < v;

            let now = Utc::now();
            let payload = {
                let mut info = UpdateInfo::new(v, url);
                info.set_fetched_at(now);
                Some(info)
            };

            self.set_last_check(now);
            Self::write_last_check_status(&p, &payload)?;
            *self.state.avail_release.borrow_mut() = payload;

            self.save()?;
            Ok(update_avail)
        };

        // if first time checking, just update the updater's timestamp, no network call
        if self.last_check().is_none() {
            self.set_last_check(Utc::now());
            self.save()?;
            Ok(false)
        } else if self.due_to_check() {
            // it's time to talk to remote server
            ask_releaser_for_update()
        } else {
            Self::read_last_check_status(&p)
                .map(|last_check_status| {
                    last_check_status
                        .map(|last_update_info| *self.current_version() < last_update_info.version)
                        .unwrap_or(false)
                })
                .or(Ok(false))
        }
    }
}

pub(super) fn default_interval() -> i64 {
    UPDATE_INTERVAL
}
