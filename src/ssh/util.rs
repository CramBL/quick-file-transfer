use std::env;

pub(super) fn get_remote_password_from_env() -> Option<String> {
    if let Some(password) = env::var(super::ENV_REMOTE_PASSWORD).ok() {
        Some(password)
    } else {
        None
    }
}
