//! Fail closed on unsafe Windows storage without invoking runtime helpers.
#![cfg(windows)]
mod support;

use silkboard_publish::Error;
use std::{fs, time::Instant};
use support::{Fixture, report};
use windows_permissions::{
    LocalBox, SecurityDescriptor,
    constants::{SeObjectType, SecurityInformation},
    utilities::current_process_sid,
    wrappers::SetNamedSecurityInfo,
};

fn grant_everyone(path: &std::path::Path) {
    let user = current_process_sid().unwrap();
    let sd: LocalBox<SecurityDescriptor> =
        format!("D:P(A;OICI;FA;;;{user})(A;OICI;FR;;;WD)")
            .parse()
            .unwrap();
    SetNamedSecurityInfo(
        path.as_os_str(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        sd.dacl(),
        None,
    )
    .unwrap();
}

#[test]
fn shared_credentials_and_directories_are_rejected() {
    for name in ["token", "state", "config.json"] {
        let f = Fixture::new();
        let args = f.args();
        grant_everyone(&f.root.join(name));
        assert_eq!(
            silkboard_publish::run(
                &args,
                serde_json::to_vec(&report()).unwrap().as_slice()
            ),
            Err(Error::PrivateFile),
            "{name}"
        );
    }
}

#[test]
fn hardlinked_credentials_and_busy_state_fail_closed() {
    let f = Fixture::new();
    let link = f.root.join("token-copy");
    fs::hard_link(f.root.join("token"), &link).unwrap();
    assert_eq!(f.run(&report()), Err(Error::PrivateFile));
    fs::remove_file(link).unwrap();
    assert_eq!(f.run(&report()), Err(Error::Network));
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&f.state_files("lock")[0])
        .unwrap();
    lock.lock().unwrap();
    let start = Instant::now();
    assert_eq!(f.run(&report()), Err(Error::Busy));
    assert!(start.elapsed().as_secs_f64() < 1.0);
}

#[test]
fn junction_directory_aliases_are_rejected() {
    let mut f = Fixture::new();
    let link = f.root.join("alias");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(f.root.join("state"))
        .status()
        .unwrap();
    assert!(status.success());
    f.config["state_dir"] = serde_json::json!(link);
    assert_eq!(f.run(&report()), Err(Error::PrivateFile));
    fs::remove_dir(link).unwrap();
}
