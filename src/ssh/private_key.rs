use super::{ENV_SSH_KEY_DIR, ENV_SSH_PRIVATE_KEY};
use anyhow::{bail, Result};

use std::{
    env, fs,
    path::{Path, PathBuf},
};

const PRIORITY_KEYS: [&str; 3] = ["ed25519", "rsa", "ecdsa"];

// Gets the path to the first file in `dir` that ends with any of the strings described in [`PRIORITY_KEYS`].
fn get_private_key_path_from_dir(dir: &Path) -> Result<PathBuf> {
    debug_assert!(dir.is_dir());
    for res in fs::read_dir(dir)? {
        match res {
            Ok(dir_entry) => {
                if let Some(fname) = dir_entry.file_name().to_str() {
                    for pkeys in PRIORITY_KEYS {
                        if fname.ends_with(pkeys) {
                            return Ok(dir_entry.path());
                        }
                    }
                }
            }
            Err(e) => log::error!("{e}"),
        }
    }
    bail!(
        "Failed retrieving ssh private key from {dir}. Hint: You can point to another directory with {ENV_SSH_KEY_DIR} or directly to the private key you wish to use with {ENV_SSH_PRIVATE_KEY}",
        dir = dir.to_string_lossy()
    )
}

pub fn get_ssh_private_key_path(
    pkey_path: Option<&Path>,
    pkey_dir: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(pkey_path) = pkey_path {
        return Ok(PathBuf::from(pkey_path));
    }
    if let Some(pkey_dir) = pkey_dir {
        let pkey_dir = PathBuf::from(pkey_dir);
        return get_private_key_path_from_dir(&pkey_dir);
    }
    let home = if cfg!(windows) {
        env::var("APP_DATA").expect("No APP_DATA directory")
    } else {
        env::var("HOME").expect("No HOME directory")
    };
    let home_dot_ssh_dir = PathBuf::from(home).join(".ssh");
    get_private_key_path_from_dir(&home_dot_ssh_dir)
}
