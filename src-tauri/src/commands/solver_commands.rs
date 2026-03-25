// OptiFlow — Solver & validation command handlers

use crate::models::*;
use crate::state::AppState;
use crate::solver;
use crate::validator;
use tauri::State;

#[tauri::command]
pub fn validate_model(state: State<AppState>) -> Vec<ValidationMessage> {
    let model = state.model.lock().unwrap();
    validator::validate_model(&model)
}

#[tauri::command]
pub fn run_optimizer(state: State<AppState>, config: SolverConfig) -> SolverResult {
    let model = state.model.lock().unwrap();
    let result = solver::solve(&model, &config);
    {
        let mut last = state.last_result.lock().unwrap();
        *last = Some(result.clone());
    }
    result
}

#[tauri::command]
pub fn get_last_result(state: State<AppState>) -> Option<SolverResult> {
    state.last_result.lock().unwrap().clone()
}
