use std::env;

pub(super) fn get_remote_password_from_env() -> Option<String> {
    if let Ok(password) = env::var(super::ENV_REMOTE_PASSWORD) {
        Some(password)
    } else {
        None
    }
}
