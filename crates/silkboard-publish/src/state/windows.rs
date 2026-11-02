//! Native private NTFS storage checks, without runtime helper processes.

use crate::Error;
use std::{
    fs::{File, OpenOptions},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path, Prefix},
};
use windows_permissions::{
    LocalBox, SecurityDescriptor, Sid,
    constants::{AceType, SeObjectType, SecurityInformation},
    utilities::current_process_sid,
    wrappers::GetSecurityInfo,
};

const REPARSE_POINT: u32 = 0x400;
const BACKUP_SEMANTICS: u32 = 0x0200_0000;
const OPEN_REPARSE_POINT: u32 = 0x0020_0000;

fn private_acl(sd: &SecurityDescriptor) -> Result<(), Error> {
    let user = current_process_sid().map_err(|_| Error::PrivateFile)?;
    let system: LocalBox<Sid> =
        "S-1-5-18".parse().map_err(|_| Error::PrivateFile)?;
    let admins: LocalBox<Sid> =
        "S-1-5-32-544".parse().map_err(|_| Error::PrivateFile)?;
    let trusted = [Some(&*user), Some(&*system), Some(&*admins)];
    if !trusted.contains(&sd.owner()) {
        return Err(Error::PrivateFile);
    }
    let acl = sd.dacl().ok_or(Error::PrivateFile)?;
    for index in 0..acl.len() {
        let ace = acl.get_ace(index).ok_or(Error::PrivateFile)?;
        // Fail closed on conditional/object/unknown ACEs, even when another
        // deny might cancel a broad grant. Inheritance is checked too.
        match ace.ace_type() {
            AceType::ACCESS_DENIED_ACE_TYPE => {}
            AceType::ACCESS_ALLOWED_ACE_TYPE
                if trusted.contains(&ace.sid()) => {}
            _ => return Err(Error::PrivateFile),
        }
    }
    Ok(())
}

fn acl(file: &File) -> Result<(), Error> {
    let sd = GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )
    .map_err(|_| Error::PrivateFile)?;
    private_acl(&sd)
}

fn drive(path: &Path) -> Option<u8> {
    match path.components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                Some(letter.to_ascii_uppercase())
            }
            _ => None,
        },
        _ => None,
    }
}

fn local_path(path: &Path) -> Result<(), Error> {
    let canonical = path.canonicalize().map_err(|_| Error::PrivateFile)?;
    if !path.is_absolute()
        || drive(path).is_none()
        || drive(path) != drive(&canonical)
        || !path.components().skip(1).eq(canonical.components().skip(1))
    {
        return Err(Error::PrivateFile);
    }
    for ancestor in path.ancestors() {
        if ancestor
            .symlink_metadata()
            .map_err(|_| Error::PrivateFile)?
            .file_attributes()
            & REPARSE_POINT
            != 0
        {
            return Err(Error::PrivateFile);
        }
    }
    Ok(())
}

pub(super) fn directory(path: &Path) -> Result<(), Error> {
    local_path(path)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(BACKUP_SEMANTICS | OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| Error::PrivateFile)?;
    acl(&file)
}

pub(super) fn check(file: &File, path: &Path) -> Result<(), Error> {
    local_path(path)?;
    let named = OpenOptions::new()
        .read(true)
        .custom_flags(OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| Error::PrivateFile)?;
    let named = winapi_util::file::information(&named)
        .map_err(|_| Error::PrivateFile)?;
    let actual =
        winapi_util::file::information(file).map_err(|_| Error::PrivateFile)?;
    if actual.number_of_links() != 1
        || actual.file_attributes() & u64::from(REPARSE_POINT) != 0
        || (actual.volume_serial_number(), actual.file_index())
            != (named.volume_serial_number(), named.file_index())
    {
        return Err(Error::PrivateFile);
    }
    acl(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_permissions::LocalBox;

    #[test]
    fn owner_system_and_admins_are_the_only_allowed_grantees() {
        let user = current_process_sid().unwrap();
        for grants in ["(A;;FA;;;SY)", "(A;;FA;;;BA)", ""] {
            let sd: LocalBox<SecurityDescriptor> =
                format!("O:{user}D:(A;;FA;;;{user}){grants}")
                    .parse()
                    .unwrap();
            assert_eq!(private_acl(&sd), Ok(()));
        }
        for (owner, grants) in [
            (user.to_string(), "(A;;FR;;;WD)"),
            (user.to_string(), "(A;OICIIO;FR;;;BU)"),
            ("WD".into(), ""),
        ] {
            let sd: LocalBox<SecurityDescriptor> =
                format!("O:{owner}D:(A;;FA;;;{user}){grants}")
                    .parse()
                    .unwrap();
            assert_eq!(private_acl(&sd), Err(Error::PrivateFile));
        }
        let null_acl: LocalBox<SecurityDescriptor> =
            format!("O:{user}").parse().unwrap();
        assert_eq!(private_acl(&null_acl), Err(Error::PrivateFile));
    }
}
