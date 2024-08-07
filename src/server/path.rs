use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::bail;

use crate::config::transfer::command::DestinationMode;

/// Resolves a path that might start with a '~' or is the empty string
pub fn resolve_scp_path(remote_path: &Path) -> anyhow::Result<PathBuf> {
    // Check if we need to resolve the home directory
    // (specifying ~ or an empty string is valid and should resolve to the users home dir)
    if remote_path.starts_with("~") || remote_path.as_os_str().is_empty() {
        // Split the path to get the tilde component and the rest of the path
        let components: Vec<&str> = remote_path
            .iter()
            .map(|os_str| os_str.to_str().unwrap())
            .collect();

        // Reconstruct the path without the tilde
        let mut resolved_path = PathBuf::from(resolve_home()?);
        for component in components.iter().skip(1) {
            resolved_path.push(component);
        }

        return Ok(resolved_path);
    }

    // Return the original path if no tilde was found at the start
    Ok(remote_path.into())
}

#[cfg(not(target_os = "windows"))]
fn resolve_home() -> Result<String, env::VarError> {
    env::var("HOME")
}

#[cfg(target_os = "windows")]
fn resolve_home() -> Result<String, env::VarError> {
    // Favor $HOME even on windows
    match env::var("HOME") {
        Ok(h) => Ok(h),
        Err(_) => env::var("USERPROFILE"),
    }
}

/// Returns true if path is root
///
/// A path is root if it has a root and has no parent
pub fn is_root<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.parent().is_none() && path.has_root()
}

/// Validate that a remote path is valid for the host the server runs on.
pub fn validate_remote_path(mode: &DestinationMode, remote_path: &Path) -> anyhow::Result<PathBuf> {
    tracing::trace!("Validationg path: {remote_path:?} in {mode}");
    let resolved_path = resolve_scp_path(remote_path)?;
    tracing::trace!("Resolved {remote_path:?} -> {resolved_path:?}");
    if is_root(&resolved_path) {
        return Ok(resolved_path);
    }
    if !resolved_path.is_absolute() {
        bail!(
            "Cannot resolve '{}' to an absolute path",
            remote_path.to_string_lossy()
        );
    }
    match mode {
        DestinationMode::SingleFile => {
            if resolved_path.parent().is_some_and(|p| p.exists()) {
                Ok(resolved_path)
            } else {
                bail!(
                    "'{}' invalid path, parent directory has to be an existent directory",
                    remote_path.to_string_lossy()
                )
            }
        }
        DestinationMode::MultipleFiles => {
            if resolved_path.is_dir() {
                Ok(resolved_path)
            } else {
                bail!("transferring multiple files requires an existent destination directory")
            }
        }
        DestinationMode::RecusiveDirectory => {
            if resolved_path.is_dir()
                || (!resolved_path.is_file()
                    && resolved_path.extension().is_none()
                    && resolved_path.parent().is_some_and(|p| p.exists()))
            {
                Ok(resolved_path)
            } else {
                bail!("transferring a directory requires a destination directory")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use strum::IntoEnumIterator;
    use temp_dir::TempDir;
    use testresult::TestResult;

    const DIAG_IS_FILE_FORMAT: &str = "Destination has file-format";
    const DIAG_IS_FILE_EXISTS: &str = "Destination exists and is file";
    const DIAG_HELP_MULTI_F_DEST_DIR_REQ: &str =
        "transferring multiple files requires a destination directory";
    const DIAG_HELP_DIR_DEST_DIR_REQ: &str =
        "transferring a directory requires a destination directory";

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_resolve_scp_path_with_tilde_unix() -> TestResult {
        let home_dir = env::var("HOME")?;
        let path = PathBuf::from("~/test_dir");
        let expected_path = PathBuf::from(&home_dir).join("test_dir");
        assert_eq!(resolve_scp_path(&path)?, expected_path);
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_resolve_scp_path_empty_str() -> TestResult {
        let home_dir = env::var("HOME")?;
        let path = PathBuf::from("");
        let expected_path = PathBuf::from(&home_dir).join("");
        assert_eq!(resolve_scp_path(&path)?, expected_path);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_resolve_scp_path_with_tilde_windows() -> TestResult {
        // e.g. github actions resolve to "C:\\Users\\runneradmin\\test_dir"
        let user_profile = env::var("USERPROFILE")?;
        let path = PathBuf::from("~\\test_dir");
        let expected_path = PathBuf::from(&user_profile).join("test_dir");
        assert_eq!(resolve_scp_path(&path)?, expected_path);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_resolve_scp_path_with_empty_str() -> TestResult {
        // e.g. github actions resolve to "C:\\Users\\runneradmin\\test_dir"
        let user_profile = env::var("USERPROFILE")?;
        let path = PathBuf::from("");
        let expected_path = PathBuf::from(&user_profile).join("");
        assert_eq!(resolve_scp_path(&path)?, expected_path);
        Ok(())
    }

    /// This is valid as it resolves to a file with no file-extension in a directory that exists
    #[test]
    fn test_is_remote_path_valid_with_unix_home_no_file_extension_valid() {
        let path = PathBuf::from("~/non_existing");
        for mode in DestinationMode::iter() {
            match mode {
                DestinationMode::SingleFile => assert!(validate_remote_path(&mode, &path).is_ok()),
                DestinationMode::MultipleFiles => {
                    assert!(
                        validate_remote_path(&mode, &path).is_err(),
                        "Error: Path doesn't exist, {DIAG_HELP_MULTI_F_DEST_DIR_REQ}"
                    )
                }
                DestinationMode::RecusiveDirectory => {
                    assert!(validate_remote_path(&mode, &path).is_ok())
                }
            }
        }
    }

    /// Pointing to a directory is valid (here WITHOUT trailing '/')
    #[test]
    fn test_is_remote_path_valid_with_existing_directory_valid() -> TestResult {
        let dir = TempDir::new()?;
        let path = dir.path();
        assert!(!path.ends_with("/"));

        for mode in DestinationMode::iter() {
            match mode {
                DestinationMode::SingleFile => assert!(validate_remote_path(&mode, path).is_ok()),
                DestinationMode::MultipleFiles => {
                    assert!(validate_remote_path(&mode, path).is_ok())
                }
                DestinationMode::RecusiveDirectory => {
                    assert!(validate_remote_path(&mode, path).is_ok())
                }
            }
        }
        Ok(())
    }

    /// Pointing to a directory WITH trailing '/'
    #[test]
    fn test_is_remote_path_valid_with_existing_directory_trailing_slash() -> TestResult {
        let dir = TempDir::new()?;
        let mut dir_path = dir.path().to_str().unwrap().to_owned();
        dir_path.push('/');
        let path = PathBuf::from(dir_path);

        for mode in DestinationMode::iter() {
            match mode {
                DestinationMode::SingleFile => assert!(validate_remote_path(&mode, &path).is_ok()),
                DestinationMode::MultipleFiles => {
                    assert!(validate_remote_path(&mode, &path).is_ok())
                }
                DestinationMode::RecusiveDirectory => {
                    assert!(validate_remote_path(&mode, &path).is_ok())
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_is_remote_path_valid_with_existing_file() -> TestResult {
        let dir = TempDir::new()?;
        let path = dir.child("file.txt");
        File::create(&path)?;

        for mode in DestinationMode::iter() {
            match mode {
                DestinationMode::SingleFile => assert!(validate_remote_path(&mode, &path).is_ok()),
                DestinationMode::MultipleFiles => {
                    assert!(
                        validate_remote_path(&mode, &path).is_err(),
                        "Error: {DIAG_IS_FILE_EXISTS}, {DIAG_HELP_MULTI_F_DEST_DIR_REQ}"
                    )
                }
                DestinationMode::RecusiveDirectory => {
                    assert!(
                        validate_remote_path(&mode, &path).is_err(),
                        "Error: {DIAG_IS_FILE_EXISTS}, {DIAG_HELP_DIR_DEST_DIR_REQ}"
                    )
                }
            }
        }

        Ok(())
    }

    /// Nonexistent file in directory that exists
    #[test]
    fn test_is_remote_path_valid_with_existing_directory_but_non_existent_file() -> TestResult {
        let dir = TempDir::new()?;
        let path = dir.child("doesnt_exist.txt");
        for mode in DestinationMode::iter() {
            match mode {
                DestinationMode::SingleFile => assert!(validate_remote_path(&mode, &path).is_ok()),
                DestinationMode::MultipleFiles => {
                    assert!(
                        validate_remote_path(&mode, &path).is_err(),
                        "Error: {DIAG_IS_FILE_FORMAT}, {DIAG_HELP_MULTI_F_DEST_DIR_REQ}"
                    )
                }
                DestinationMode::RecusiveDirectory => {
                    assert!(
                        validate_remote_path(&mode, &path).is_err(),
                        "Error: {DIAG_IS_FILE_FORMAT}, {DIAG_HELP_DIR_DEST_DIR_REQ}"
                    )
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_is_remote_path_valid_with_non_absolute_path() {
        let path = PathBuf::from(
            "dsj764j7654j96h6ybvjihsbd4747cbds77r44fdsf9e4b4h6f0qxlmusghd7ahndcjsahf2sad",
        ); // Sure hope no one has this file in current dir
        for mode in DestinationMode::iter() {
            match mode {
                DestinationMode::SingleFile => assert!(
                    validate_remote_path(&mode, &path).is_err(),
                    "Err: Cannot resolve to absolute path"
                ),
                DestinationMode::MultipleFiles => {
                    assert!(
                        validate_remote_path(&mode, &path).is_err(),
                        "Err: Cannot resolve to absolute path"
                    )
                }
                DestinationMode::RecusiveDirectory => {
                    assert!(
                        validate_remote_path(&mode, &path).is_err(),
                        "Err: Cannot resolve to absolute path"
                    )
                }
            }
        }
    }

    /// https://github.com/CramBL/quick-file-transfer/issues/37
    #[test]
    fn test_validate_remote_path_with_root_issue37() {
        let path = PathBuf::from("/");
        for mode in DestinationMode::iter() {
            match mode {
                DestinationMode::SingleFile => assert!(validate_remote_path(&mode, &path).is_ok()),
                DestinationMode::MultipleFiles => {
                    assert!(validate_remote_path(&mode, &path).is_ok())
                }
                DestinationMode::RecusiveDirectory => {
                    assert!(validate_remote_path(&mode, &path).is_ok())
                }
            }
        }
    }
}
