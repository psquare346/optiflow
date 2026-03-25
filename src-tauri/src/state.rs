// OptiFlow — Application state

use std::sync::Mutex;
use crate::models::*;

pub struct AppState {
    pub model: Mutex<SupplyChainModel>,
    pub aliases: Mutex<AliasMap>,
    pub last_result: Mutex<Option<SolverResult>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            model: Mutex::new(SupplyChainModel::new("OptiFlow Model", 4)),
            aliases: Mutex::new(AliasMap::default()),
            last_result: Mutex::new(None),
        }
    }
}
