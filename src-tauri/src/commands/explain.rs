// OptiFlow — Explain command handler

use crate::models::*;
use crate::state::AppState;
use crate::explainer;
use tauri::State;

#[tauri::command]
pub fn explain_decision(state: State<AppState>, question: String) -> explainer::ExplainResponse {
    let model = state.model.lock().unwrap();
    let last = state.last_result.lock().unwrap();
    match last.as_ref() {
        Some(result) => explainer::explain(&question, &model, result),
        None => explainer::ExplainResponse {
            intent: "error".into(),
            answer: "No optimization results available. Please run the optimizer first.".into(),
            data_points: vec![],
            suggestions: vec!["Go to Run Optimizer and click 🚀 Run Optimizer".into()],
        },
    }
}
