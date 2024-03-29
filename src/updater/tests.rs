use self::releaser::tests::setup_mock_server;
// #[cfg(not(feature = "ci"))]
use self::releaser::GithubReleaser;
use self::releaser::MOCK_RELEASER_REPO_NAME;
use super::*;
use std::ffi::OsStr;
use std::{thread, time};
use tempfile::Builder;
const VERSION_TEST: &str = "0.10.5";
const VERSION_TEST_NEW: &str = "0.11.1"; // should match what the mock server replies for new version.

#[test]
fn it_tests_settings_filename() {
    setup_workflow_env_vars(true);
    let updater_state_fn = Updater::<GithubReleaser>::build_data_fn().unwrap();
    assert_eq!(
        "workflow.B0AC54EC-601C-YouForgotTo___Name_Your_Own_Work_flow_-updater.json",
        updater_state_fn.file_name().unwrap().to_str().unwrap()
    );
}

#[test]
fn it_ignores_saved_version_after_an_upgrade_async() {
    // Make sure a freshly upgraded workflow does not use version info from saved state
    setup_workflow_env_vars(true);
    let _m = setup_mock_server(200);
    first_check_after_installing_workflow();

    {
        // Next check it reports a new version since mock server has a release for us
        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        updater.set_interval(0);
        updater.init().expect("couldn't init worker");

        assert!(updater.update_ready().expect("couldn't check for update"));
        assert_eq!(VERSION_TEST, format!("{}", updater.current_version()));
    }

    // Mimic the upgrade process by bumping the version
    StdEnv::set_var("alfred_workflow_version", VERSION_TEST_NEW);
    {
        let updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        // Updater should pick up new version rather than using saved one
        assert_eq!(VERSION_TEST_NEW, format!("{}", updater.current_version()));
        updater.init().expect("couldn't init worker");
        // No more updates
        assert!(!updater.update_ready().expect("couldn't check for update"));
    }
}

#[test]
#[should_panic(
    expected = "HTTP status client error (400 Bad Request) for url (http://127.0.0.1:1234/releases/latest)"
)]
fn it_handles_server_error_async() {
    setup_workflow_env_vars(true);
    first_check_after_installing_workflow();

    let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
    // Next check will be immediate
    updater.set_interval(0);
    updater.init().expect("couldn't init worker");
    let _m = setup_mock_server(400);
    // This should panic with a BadRequest (400) error.
    updater.update_ready().unwrap();
}

#[test]
fn it_caches_async_workers_payload() {
    setup_workflow_env_vars(true);

    first_check_after_installing_workflow();
    let _m = setup_mock_server(200);
    let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
    // Next check will be immediate
    updater.set_interval(0);
    updater.init().expect("couldn't init worker");
    assert!(updater.update_ready().expect("couldn't check for update"),);

    // Consequent calls to update_ready should cache the payload.
    let _m = setup_mock_server(400);
    assert!(updater.update_ready().expect("couldn't check for update"),);
    assert!(updater.update_ready().expect("couldn't check for update"),);
    assert!(updater.update_ready().expect("couldn't check for update"),);

    {
        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        // Next check will be immediate
        updater.set_interval(0);
        updater.init().expect("couldn't init worker");
        assert!(updater.update_ready().is_err());
    }
}

#[test]
fn it_get_latest_info_from_releaser() {
    setup_workflow_env_vars(true);
    let _m = setup_mock_server(200);

    {
        first_check_after_installing_workflow();
        // Blocking
        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        // Next check will be immediate
        updater.set_interval(0);
        updater.init().expect("couldn't init worker");

        assert!(updater
            .update_ready()
            .expect("Blocking: couldn't check for update"));
    }
    {
        // Non-blocking
        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        // Next check will be immediate
        updater.set_interval(0);
        // Start async worker
        updater.init().expect("couldn't init worker");
        let wait = time::Duration::from_millis(500);
        thread::sleep(wait);

        assert!(updater
            .try_update_ready()
            .expect("Non-blocking: couldn't check for update"));
    }
}

#[allow(clippy::cast_possible_wrap)]
#[test]
fn it_does_one_network_call_per_interval() {
    {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);
        let wait_time = 1;

        first_check_after_installing_workflow();

        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        // Next check will be immediate
        updater.set_interval(0);
        updater.init().expect("couldn't init worker");

        // Next update_ready will make a network call
        assert!(updater.update_ready().expect("couldn't check for update"));

        // Increase interval
        updater.set_interval(wait_time as i64);
        assert!(!updater.due_to_check());

        // make mock server return error. This way we can test that no network call was made
        // assuming Updater can read its cache file successfully
        let _m = setup_mock_server(503);
        let t = updater.update_ready();
        assert!(t.is_ok());
        // Make sure we still report update is ready
        assert!(t.unwrap());

        // Now we test that after interval has passed we will make a call
        let two_sec = time::Duration::from_secs(wait_time);
        thread::sleep(two_sec);
        {
            let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
            updater.set_interval(wait_time as i64);
            updater.init().expect("couldn't init worker");
            assert!(updater.due_to_check());

            // Since server is returning error, update_ready() should fail.
            let t = updater.update_ready();
            assert!(t.is_err());
        }
        {
            // Just making sure the next call will go through and return expected results.
            let _m = setup_mock_server(200);
            let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
            // Next check will be immediate
            updater.set_interval(0);
            updater.init().expect("couldn't init worker");
            assert!(updater.due_to_check());

            // Since server is ok, update_ready() should work.
            let t = updater.update_ready();
            assert!(t.is_ok());
            assert!(updater.update_ready().expect("couldn't check for update"));
        }
    }
}

#[test]
fn it_tests_download() {
    setup_workflow_env_vars(true);
    let _m = setup_mock_server(200);
    first_check_after_installing_workflow();

    let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");

    // Next check will be immediate
    updater.set_interval(0);
    // Force current version to be really old.
    updater.set_version("0.0.1");
    updater.init().expect("couldn't init worker");

    // New update is available
    assert!(updater.update_ready().expect("couldn't check for update"));

    let download_fn = updater.download_latest();
    assert!(download_fn.is_ok());
    assert_eq!(
        "latest_release_YouForgotTo___Name_Your_Own_Work_flow_.alfredworkflow",
        download_fn
            .unwrap()
            .file_name()
            .expect("couldn't get download file name")
            .to_str()
            .expect("impossible?!")
    );
}

#[test]
#[should_panic(expected = "no release info")]
fn it_doesnt_download_without_release_info() {
    setup_workflow_env_vars(true);
    let _m = setup_mock_server(200);
    first_check_after_installing_workflow();

    let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
    updater.set_interval(864_000);

    assert!(!updater.due_to_check());
    updater.init().expect("couldn't init worker");

    assert!(updater.download_latest().is_err());

    // Since check time is due yet, following will just read cache without
    // getting any release info, hence the last line should panic
    assert!(!updater.update_ready().expect("couldn't check for update"));
    updater.download_latest().unwrap();
}

#[test]
fn it_downloads_after_getting_release_info() {
    setup_workflow_env_vars(true);
    let _m = setup_mock_server(200);
    first_check_after_installing_workflow();

    let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
    updater.set_interval(0);
    updater.init().expect("couldn't init worker");
    assert!(updater.download_latest().is_err());

    assert!(updater.update_ready().expect("couldn't check for update"));
    assert!(updater.download_latest().is_ok());
}

#[test]
fn it_tests_async_updates_1() {
    //
    setup_workflow_env_vars(true);
    let _m = setup_mock_server(200);
    first_check_after_installing_workflow();

    let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
    // Next check will be immediate
    updater.set_interval(0);
    updater.init().expect("couldn't init worker");
    assert!(updater.update_ready().expect("couldn't check for update"));
}

#[test]
fn it_tests_async_updates_2() {
    // This test will only spawn a thread once.
    // Second call will use a cache since it's not due to check.
    setup_workflow_env_vars(true);
    let _m = setup_mock_server(200);
    first_check_after_installing_workflow();

    let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");

    // Next check will spawn a thread. There should be an update avail. from mock server.
    updater.set_interval(0);
    updater.init().expect("couldn't init worker");
    updater.update_ready().expect("chouldn't check for update");

    // make mock server return error. This way we can test that no network call was made
    // assuming Updater can read its cache file successfully
    let _m = setup_mock_server(503);
    // Increase interval
    updater.set_interval(86400);

    assert!(updater.update_ready().expect("couldn't check for update"));
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
        StdEnv::set_var("alfred_workflow_version", VERSION_TEST);
    }
    path
}

fn first_check_after_installing_workflow() {
    // since the first check after workflow installation by user will return no update available,
    // we need to run it at the beginning of some tests
    let _m = setup_mock_server(200);

    let updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
    assert_eq!(VERSION_TEST, format!("{}", updater.current_version()));

    updater.init().expect("couldn't init worker");

    // First update_ready is always false.
    assert!(!updater.update_ready().expect("couldn't check for update"));
}
