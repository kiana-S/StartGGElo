use std::path::PathBuf;

pub struct AppState {
    pub config_dir: PathBuf,
    pub auth_token: String,
}
