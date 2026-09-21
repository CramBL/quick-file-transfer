use std::env;

pub(super) fn get_remote_password_from_env() -> Option<String> {
    env::var(super::ENV_REMOTE_PASSWORD).ok()
}
