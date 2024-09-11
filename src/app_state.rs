use crate::config::Config;
use crate::database::PostgreDatabase;

pub struct AppState {
    pub db: PostgreDatabase,
    pub config: Config,
}
