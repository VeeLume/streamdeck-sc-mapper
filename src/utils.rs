use std::sync::{ MutexGuard };

use crate::{ plugin::{ APP_STATE }, state::AppState };

pub fn get_locked_app_state<'a>() -> Result<MutexGuard<'a, AppState>, &'a str> {
    match APP_STATE.get() {
        Some(app_state) => {
            match app_state.lock() {
                Ok(state) => Ok(state),
                Err(_) => Err("AppState poisoned"),
            }
        }
        None => Err("AppState not initialized"),
    }
}
