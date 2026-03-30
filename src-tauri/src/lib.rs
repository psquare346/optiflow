// Core modules — shared by both Tauri and web server
pub mod models;
pub mod solver;
pub mod state;
pub mod validator;
pub mod explainer;

// Tauri-specific modules (commands use #[tauri::command] macro)
#[cfg(feature = "tauri-app")]
mod commands;

use state::AppState;

#[cfg(feature = "tauri-app")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Location CRUD
            commands::get_locations,
            commands::add_location,
            commands::delete_location,
            // Product CRUD
            commands::get_products,
            commands::add_product,
            commands::delete_product,
            // Resource CRUD
            commands::get_resources,
            commands::add_resource,
            commands::delete_resource,
            // Transport Lane CRUD
            commands::get_transport_lanes,
            commands::add_transport_lane,
            commands::delete_transport_lane,
            // Supplier CRUD (NEW)
            commands::get_suppliers,
            commands::add_supplier,
            commands::delete_supplier,
            // Customer CRUD (NEW)
            commands::get_customers,
            commands::add_customer,
            commands::delete_customer,
            // Demand CRUD
            commands::get_demands,
            commands::add_demand,
            commands::delete_demand,
            // Product-Location CRUD
            commands::get_product_locations,
            commands::add_product_location,
            commands::delete_product_location,
            // Product-Resource CRUD
            commands::get_product_resources,
            commands::add_product_resource,
            commands::delete_product_resource,
            // BOM CRUD
            commands::get_bom_entries,
            commands::add_bom_entry,
            // Sourcing Rules
            commands::get_sourcing_rules,
            commands::add_sourcing_rule,
            // Planned Receipts (NEW)
            commands::get_planned_receipts,
            commands::add_planned_receipt,
            commands::delete_planned_receipt,
            // Product Prices (NEW)
            commands::get_product_prices,
            commands::add_product_price,
            // Solver
            commands::validate_model,
            commands::run_optimizer,
            commands::get_last_result,
            // Aliases
            commands::get_aliases,
            commands::set_alias,
            // Demo
            commands::load_demo_data,
            // Explainer
            commands::explain_decision,
        ])
        .run(tauri::generate_context!())
        .expect("error running OptiFlow");
}

