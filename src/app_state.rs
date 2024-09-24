use crate::config::Config;
use crate::database::PostgreDatabase;
use crate::external::External;

pub struct AppState {
    pub db: PostgreDatabase,
    pub ext: External,
    pub config: Config,
}
