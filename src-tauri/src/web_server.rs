// OptiFlow — Axum Web Server (Docker / cloud deployment)
// Mirrors every Tauri command as a JSON HTTP endpoint.
// Reuses: models, solver, validator, state, and demo data logic.

use axum::{
    Json, Router,
    extract::State as AxState,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;

// Import the library crate
use optiflow_lib::models::*;
use optiflow_lib::solver;
use optiflow_lib::validator;
use optiflow_lib::explainer;

// ─── Shared state (same as Tauri AppState but with Arc) ─────

use std::sync::Mutex;

pub struct WebState {
    pub model: Mutex<SupplyChainModel>,
    pub aliases: Mutex<AliasMap>,
    pub last_result: Mutex<Option<SolverResult>>,
}

impl Default for WebState {
    fn default() -> Self {
        Self {
            model: Mutex::new(SupplyChainModel::new("OptiFlow Model", 4)),
            aliases: Mutex::new(AliasMap::default()),
            last_result: Mutex::new(None),
        }
    }
}

type AppState = Arc<WebState>;

// ─── Main ───────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let state: AppState = Arc::new(WebState::default());

    let api = Router::new()
        // Location CRUD
        .route("/get_locations", get(get_locations))
        .route("/add_location", post(add_location))
        .route("/delete_location", post(delete_location))
        // Product CRUD
        .route("/get_products", get(get_products))
        .route("/add_product", post(add_product))
        .route("/delete_product", post(delete_product))
        // Resource CRUD
        .route("/get_resources", get(get_resources))
        .route("/add_resource", post(add_resource))
        .route("/delete_resource", post(delete_resource))
        // Transport Lane CRUD
        .route("/get_transport_lanes", get(get_transport_lanes))
        .route("/add_transport_lane", post(add_transport_lane))
        .route("/delete_transport_lane", post(delete_transport_lane))
        // Supplier CRUD
        .route("/get_suppliers", get(get_suppliers))
        .route("/add_supplier", post(add_supplier))
        .route("/delete_supplier", post(delete_supplier))
        // Customer CRUD
        .route("/get_customers", get(get_customers))
        .route("/add_customer", post(add_customer))
        .route("/delete_customer", post(delete_customer))
        // Demand CRUD
        .route("/get_demands", get(get_demands))
        .route("/add_demand", post(add_demand))
        .route("/delete_demand", post(delete_demand))
        // Product-Location CRUD
        .route("/get_product_locations", get(get_product_locations))
        .route("/add_product_location", post(add_product_location))
        .route("/delete_product_location", post(delete_product_location))
        // Product-Resource CRUD
        .route("/get_product_resources", get(get_product_resources))
        .route("/add_product_resource", post(add_product_resource))
        .route("/delete_product_resource", post(delete_product_resource))
        // BOM CRUD
        .route("/get_bom_entries", get(get_bom_entries))
        .route("/add_bom_entry", post(add_bom_entry))
        // Sourcing Rules
        .route("/get_sourcing_rules", get(get_sourcing_rules))
        .route("/add_sourcing_rule", post(add_sourcing_rule))
        // Planned Receipts
        .route("/get_planned_receipts", get(get_planned_receipts))
        .route("/add_planned_receipt", post(add_planned_receipt))
        .route("/delete_planned_receipt", post(delete_planned_receipt))
        // Product Prices
        .route("/get_product_prices", get(get_product_prices))
        .route("/add_product_price", post(add_product_price))
        // Solver
        .route("/validate_model", get(validate_model))
        .route("/run_optimizer", post(run_optimizer))
        .route("/get_last_result", get(get_last_result))
        // Aliases
        .route("/get_aliases", get(get_aliases))
        .route("/set_alias", post(set_alias))
        // Demo
        .route("/load_demo_data", post(load_demo_data))
        // Explainer
        .route("/explain_decision", post(explain_decision));

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new("dist"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 OptiFlow web server running at http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ═══════════════════════════════════════════════════════════════
// LOCATION CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_locations(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.locations.clone())
}

async fn add_location(AxState(state): AxState<AppState>, Json(location): Json<Location>) -> StatusCode {
    state.model.lock().unwrap().locations.push(location);
    StatusCode::OK
}

#[derive(Deserialize)]
struct IdPayload { id: String }

async fn delete_location(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().locations.retain(|l| l.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_products(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.products.clone())
}

async fn add_product(AxState(state): AxState<AppState>, Json(product): Json<Product>) -> StatusCode {
    state.model.lock().unwrap().products.push(product);
    StatusCode::OK
}

async fn delete_product(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().products.retain(|x| x.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// RESOURCE CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_resources(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.resources.clone())
}

async fn add_resource(AxState(state): AxState<AppState>, Json(resource): Json<Resource>) -> StatusCode {
    state.model.lock().unwrap().resources.push(resource);
    StatusCode::OK
}

async fn delete_resource(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().resources.retain(|x| x.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// TRANSPORT LANE CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_transport_lanes(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.transport_lanes.clone())
}

async fn add_transport_lane(AxState(state): AxState<AppState>, Json(lane): Json<TransportLane>) -> StatusCode {
    state.model.lock().unwrap().transport_lanes.push(lane);
    StatusCode::OK
}

async fn delete_transport_lane(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().transport_lanes.retain(|x| x.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// SUPPLIER CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_suppliers(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.suppliers.clone())
}

async fn add_supplier(AxState(state): AxState<AppState>, Json(supplier): Json<Supplier>) -> StatusCode {
    state.model.lock().unwrap().suppliers.push(supplier);
    StatusCode::OK
}

async fn delete_supplier(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().suppliers.retain(|x| x.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// CUSTOMER CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_customers(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.customers.clone())
}

async fn add_customer(AxState(state): AxState<AppState>, Json(customer): Json<Customer>) -> StatusCode {
    state.model.lock().unwrap().customers.push(customer);
    StatusCode::OK
}

async fn delete_customer(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().customers.retain(|x| x.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// DEMAND CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_demands(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.demands.clone())
}

async fn add_demand(AxState(state): AxState<AppState>, Json(demand): Json<Demand>) -> StatusCode {
    state.model.lock().unwrap().demands.push(demand);
    StatusCode::OK
}

async fn delete_demand(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().demands.retain(|x| x.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT-LOCATION CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_product_locations(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.product_locations.clone())
}

async fn add_product_location(AxState(state): AxState<AppState>, Json(pl): Json<ProductLocation>) -> StatusCode {
    state.model.lock().unwrap().product_locations.push(pl);
    StatusCode::OK
}

#[derive(Deserialize)]
struct ProductLocationKey { product_id: String, location_id: String }

async fn delete_product_location(AxState(state): AxState<AppState>, Json(p): Json<ProductLocationKey>) -> StatusCode {
    state.model.lock().unwrap().product_locations.retain(|pl| {
        !(pl.product_id == p.product_id && pl.location_id == p.location_id)
    });
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT-RESOURCE CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_product_resources(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.product_resources.clone())
}

async fn add_product_resource(AxState(state): AxState<AppState>, Json(pr): Json<ProductResource>) -> StatusCode {
    state.model.lock().unwrap().product_resources.push(pr);
    StatusCode::OK
}

#[derive(Deserialize)]
struct ProductResourceKey { product_id: String, resource_id: String }

async fn delete_product_resource(AxState(state): AxState<AppState>, Json(p): Json<ProductResourceKey>) -> StatusCode {
    state.model.lock().unwrap().product_resources.retain(|pr| {
        !(pr.product_id == p.product_id && pr.resource_id == p.resource_id)
    });
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// BOM CRUD
// ═══════════════════════════════════════════════════════════════

async fn get_bom_entries(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.bom_entries.clone())
}

async fn add_bom_entry(AxState(state): AxState<AppState>, Json(entry): Json<BomEntry>) -> StatusCode {
    state.model.lock().unwrap().bom_entries.push(entry);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// SOURCING RULES
// ═══════════════════════════════════════════════════════════════

async fn get_sourcing_rules(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.sourcing_rules.clone())
}

async fn add_sourcing_rule(AxState(state): AxState<AppState>, Json(rule): Json<SourcingRule>) -> StatusCode {
    state.model.lock().unwrap().sourcing_rules.push(rule);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// PLANNED RECEIPTS
// ═══════════════════════════════════════════════════════════════

async fn get_planned_receipts(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.planned_receipts.clone())
}

async fn add_planned_receipt(AxState(state): AxState<AppState>, Json(receipt): Json<PlannedReceipt>) -> StatusCode {
    state.model.lock().unwrap().planned_receipts.push(receipt);
    StatusCode::OK
}

async fn delete_planned_receipt(AxState(state): AxState<AppState>, Json(p): Json<IdPayload>) -> StatusCode {
    state.model.lock().unwrap().planned_receipts.retain(|r| r.id != p.id);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT PRICES
// ═══════════════════════════════════════════════════════════════

async fn get_product_prices(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(model.product_prices.clone())
}

async fn add_product_price(AxState(state): AxState<AppState>, Json(price): Json<ProductPrice>) -> StatusCode {
    state.model.lock().unwrap().product_prices.push(price);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// SOLVER & VALIDATION
// ═══════════════════════════════════════════════════════════════

async fn validate_model(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    Json(validator::validate_model(&model))
}

#[derive(Deserialize)]
struct RunOptimizerPayload { config: SolverConfig }

async fn run_optimizer(AxState(state): AxState<AppState>, Json(payload): Json<RunOptimizerPayload>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    let result = solver::solve(&model, &payload.config);
    drop(model); // release model lock before acquiring last_result lock
    {
        let mut last = state.last_result.lock().unwrap();
        *last = Some(result.clone());
    }
    Json(result)
}

async fn get_last_result(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let last = state.last_result.lock().unwrap();
    Json(last.clone())
}

// ═══════════════════════════════════════════════════════════════
// ALIASES
// ═══════════════════════════════════════════════════════════════

async fn get_aliases(AxState(state): AxState<AppState>) -> impl IntoResponse {
    let aliases = state.aliases.lock().unwrap();
    Json(aliases.clone())
}

#[derive(Deserialize)]
struct SetAliasPayload { internal_name: String, display_name: String }

async fn set_alias(AxState(state): AxState<AppState>, Json(p): Json<SetAliasPayload>) -> StatusCode {
    state.aliases.lock().unwrap().set_alias(&p.internal_name, &p.display_name);
    StatusCode::OK
}

// ═══════════════════════════════════════════════════════════════
// DEMO DATA (inlined — same logic as commands/demo.rs)
// ═══════════════════════════════════════════════════════════════

async fn load_demo_data(AxState(state): AxState<AppState>) -> impl IntoResponse {
    use uuid::Uuid;

    let mut model = state.model.lock().unwrap();
    *model = SupplyChainModel::new("Semiconductor Demo", 8);

    // ─── Locations ───
    model.locations.push(Location::new("LOC-TW-FAB", "Taiwan Fab (TSMC-style)", LocationType::Plant, "Taiwan", "APAC"));
    model.locations.push(Location::new("LOC-TX-FAB", "Texas Assembly", LocationType::Plant, "USA", "Americas"));
    model.locations.push(Location::new("LOC-DE-WH", "Germany Warehouse", LocationType::Warehouse, "Germany", "EMEA"));
    model.locations.push(Location::new("LOC-BMW", "BMW Munich", LocationType::Customer, "Germany", "EMEA"));
    model.locations.push(Location::new("LOC-TESLA", "Tesla Austin", LocationType::Customer, "USA", "Americas"));

    // ─── Products ───
    model.products.push(Product::new("PROD-A14", "A14 Processor", ProductType::FinishedGood, "EA", 0.92));
    model.products.push(Product::new("PROD-M2", "M2 Chip", ProductType::FinishedGood, "EA", 0.88));
    model.products.push(Product::new("PROD-WAFER", "300mm Silicon Wafer", ProductType::RawMaterial, "EA", 1.0));

    // ─── Resources ───
    model.resources.push(Resource::new("RES-TW-LINE1", "Taiwan Fab Line 1", "LOC-TW-FAB", 50000.0, 12.0));
    model.resources.push(Resource::new("RES-TX-LINE1", "Texas Fab Line 1", "LOC-TX-FAB", 30000.0, 15.0));

    // ─── Suppliers ───
    model.suppliers.push(Supplier::new("SUP-SUMCO", "SUMCO Corp", "Japan"));
    model.suppliers.push(Supplier::new("SUP-SHIN", "Shin-Etsu Chemical", "Japan"));

    // ─── Customers ───
    let mut cust_bmw = Customer::new("CUST-BMW", "BMW AG", "Germany");
    cust_bmw.priority = 1;
    cust_bmw.service_level_target = 0.98;
    model.customers.push(cust_bmw);
    let mut cust_tesla = Customer::new("CUST-TESLA", "Tesla Inc", "USA");
    cust_tesla.priority = 1;
    cust_tesla.service_level_target = 0.95;
    model.customers.push(cust_tesla);

    // ─── Product-Location ───
    let mut pl_tw_a14 = ProductLocation::new("PROD-A14", "LOC-TW-FAB");
    pl_tw_a14.can_produce = true; pl_tw_a14.initial_inventory = 5000.0;
    pl_tw_a14.safety_stock = 2000.0; pl_tw_a14.holding_cost_per_unit = 0.50;
    pl_tw_a14.safety_stock_violation_cost = 15.0;
    model.product_locations.push(pl_tw_a14);

    let mut pl_tw_m2 = ProductLocation::new("PROD-M2", "LOC-TW-FAB");
    pl_tw_m2.can_produce = true; pl_tw_m2.initial_inventory = 3000.0;
    pl_tw_m2.safety_stock = 1500.0; pl_tw_m2.holding_cost_per_unit = 0.60;
    pl_tw_m2.safety_stock_violation_cost = 15.0;
    model.product_locations.push(pl_tw_m2);

    let mut pl_tw_wafer = ProductLocation::new("PROD-WAFER", "LOC-TW-FAB");
    pl_tw_wafer.can_procure = true; pl_tw_wafer.procurement_cost = 3.0;
    pl_tw_wafer.initial_inventory = 80000.0; pl_tw_wafer.holding_cost_per_unit = 0.10;
    model.product_locations.push(pl_tw_wafer);

    let mut pl_tx_a14 = ProductLocation::new("PROD-A14", "LOC-TX-FAB");
    pl_tx_a14.can_produce = true; pl_tx_a14.initial_inventory = 2000.0;
    pl_tx_a14.safety_stock = 1000.0; pl_tx_a14.holding_cost_per_unit = 0.55;
    pl_tx_a14.yield_rate_override = Some(0.90);
    model.product_locations.push(pl_tx_a14);

    let mut pl_de_a14 = ProductLocation::new("PROD-A14", "LOC-DE-WH");
    pl_de_a14.initial_inventory = 8000.0; pl_de_a14.safety_stock = 3000.0;
    pl_de_a14.max_stock = 50000.0; pl_de_a14.holding_cost_per_unit = 0.40;
    model.product_locations.push(pl_de_a14);

    let mut pl_de_m2 = ProductLocation::new("PROD-M2", "LOC-DE-WH");
    pl_de_m2.initial_inventory = 4000.0; pl_de_m2.safety_stock = 2000.0;
    pl_de_m2.max_stock = 30000.0; pl_de_m2.holding_cost_per_unit = 0.45;
    model.product_locations.push(pl_de_m2);

    model.product_locations.push(ProductLocation::new("PROD-A14", "LOC-BMW"));
    model.product_locations.push(ProductLocation::new("PROD-M2", "LOC-BMW"));
    model.product_locations.push(ProductLocation::new("PROD-A14", "LOC-TESLA"));

    // ─── Product-Resource ───
    let mut pr_tw_a14 = ProductResource::new("PROD-A14", "RES-TW-LINE1", "LOC-TW-FAB");
    pr_tw_a14.consumption_rate = 1.0;
    model.product_resources.push(pr_tw_a14);
    let mut pr_tw_m2 = ProductResource::new("PROD-M2", "RES-TW-LINE1", "LOC-TW-FAB");
    pr_tw_m2.consumption_rate = 1.2;
    model.product_resources.push(pr_tw_m2);
    let mut pr_tx_a14 = ProductResource::new("PROD-A14", "RES-TX-LINE1", "LOC-TX-FAB");
    pr_tx_a14.consumption_rate = 1.0;
    model.product_resources.push(pr_tx_a14);

    // ─── BOM ───
    model.bom_entries.push(BomEntry { output_product_id: "PROD-A14".into(), input_product_id: "PROD-WAFER".into(), quantity_per: 0.5, yield_rate: 1.0, location_id: None });
    model.bom_entries.push(BomEntry { output_product_id: "PROD-M2".into(), input_product_id: "PROD-WAFER".into(), quantity_per: 0.8, yield_rate: 1.0, location_id: None });

    // ─── Transport Lanes ───
    let mut tl1 = TransportLane::new("LOC-TW-FAB", "LOC-DE-WH", TransportMode::Ocean, 0.85, 21);
    tl1.lead_time_periods = 1; model.transport_lanes.push(tl1);
    let mut tl2 = TransportLane::new("LOC-TW-FAB", "LOC-TESLA", TransportMode::Ocean, 1.20, 18);
    tl2.lead_time_periods = 1; tl2.tariff_rate = 0.25; model.transport_lanes.push(tl2);
    let mut tl3 = TransportLane::new("LOC-TX-FAB", "LOC-TESLA", TransportMode::Truck, 0.30, 2);
    tl3.lead_time_periods = 0; model.transport_lanes.push(tl3);
    let mut tl4 = TransportLane::new("LOC-TX-FAB", "LOC-DE-WH", TransportMode::Ocean, 1.50, 14);
    tl4.lead_time_periods = 1; model.transport_lanes.push(tl4);
    let mut tl5 = TransportLane::new("LOC-DE-WH", "LOC-BMW", TransportMode::Truck, 0.15, 1);
    tl5.lead_time_periods = 0; model.transport_lanes.push(tl5);

    // ─── Sourcing Rules ───
    let sr = |st, pid: &str, from: &str, to: Option<&str>, rid: Option<&str>, prio, quota| SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: st,
        product_id: pid.into(), from_location_id: from.into(),
        to_location_id: to.map(|s| s.into()), resource_id: rid.map(|s| s.into()),
        priority: prio, quota_percentage: quota, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    };
    model.sourcing_rules.push(sr(SourcingType::Production, "PROD-A14", "LOC-TW-FAB", None, Some("RES-TW-LINE1"), 1, 0.7));
    model.sourcing_rules.push(sr(SourcingType::Production, "PROD-M2",  "LOC-TW-FAB", None, Some("RES-TW-LINE1"), 1, 1.0));
    model.sourcing_rules.push(sr(SourcingType::Production, "PROD-A14", "LOC-TX-FAB", None, Some("RES-TX-LINE1"), 2, 0.3));
    model.sourcing_rules.push(sr(SourcingType::Transport, "PROD-A14", "LOC-TW-FAB", Some("LOC-DE-WH"),  None, 1, 1.0));
    model.sourcing_rules.push(sr(SourcingType::Transport, "PROD-M2",  "LOC-TW-FAB", Some("LOC-DE-WH"),  None, 1, 1.0));
    model.sourcing_rules.push(sr(SourcingType::Transport, "PROD-A14", "LOC-TW-FAB", Some("LOC-TESLA"), None, 2, 1.0));
    model.sourcing_rules.push(sr(SourcingType::Transport, "PROD-A14", "LOC-TX-FAB", Some("LOC-TESLA"), None, 1, 1.0));
    model.sourcing_rules.push(sr(SourcingType::Transport, "PROD-A14", "LOC-TX-FAB", Some("LOC-DE-WH"),  None, 2, 1.0));
    model.sourcing_rules.push(sr(SourcingType::Transport, "PROD-A14", "LOC-DE-WH",  Some("LOC-BMW"),   None, 1, 1.0));
    model.sourcing_rules.push(sr(SourcingType::Transport, "PROD-M2",  "LOC-DE-WH",  Some("LOC-BMW"),   None, 1, 1.0));

    // ─── Planned Receipts ───
    let pr = |pid: &str, lid: &str, period, qty, rt, src: Option<&str>| PlannedReceipt {
        id: Uuid::new_v4().to_string(), product_id: pid.into(), location_id: lid.into(),
        period, quantity: qty, receipt_type: rt, source: src.map(|s| s.into()), is_firm: true,
    };
    model.planned_receipts.push(pr("PROD-WAFER", "LOC-TW-FAB", 0, 40000.0, ReceiptType::PurchaseOrder, Some("SUP-SUMCO")));
    model.planned_receipts.push(pr("PROD-WAFER", "LOC-TW-FAB", 2, 40000.0, ReceiptType::PurchaseOrder, Some("SUP-SHIN")));
    model.planned_receipts.push(pr("PROD-A14",   "LOC-DE-WH",  1, 12000.0, ReceiptType::InTransit,      Some("LOC-TW-FAB")));
    model.planned_receipts.push(pr("PROD-M2",    "LOC-DE-WH",  1, 8000.0,  ReceiptType::InTransit,      Some("LOC-TW-FAB")));
    model.planned_receipts.push(pr("PROD-A14",   "LOC-TX-FAB", 0, 5000.0,  ReceiptType::ProductionOrder, None));
    model.planned_receipts.push(pr("PROD-A14",   "LOC-TESLA",  0, 10000.0, ReceiptType::InTransit,      Some("LOC-TX-FAB")));

    // ─── Product Prices ───
    model.product_prices.push(ProductPrice { product_id: "PROD-A14".into(), location_id: None, customer_id: None, period: None, price_per_unit: 45.0 });
    model.product_prices.push(ProductPrice { product_id: "PROD-M2".into(),  location_id: None, customer_id: None, period: None, price_per_unit: 55.0 });

    // ─── Demand ───
    let demand_data = vec![
        ("PROD-A14", "LOC-BMW",   "CUST-BMW",   vec![8000.0, 9000.0, 10000.0, 12000.0, 14000.0, 12000.0, 10000.0, 9000.0]),
        ("PROD-M2",  "LOC-BMW",   "CUST-BMW",   vec![5000.0, 5500.0, 6000.0, 7000.0, 8000.0, 7000.0, 6000.0, 5000.0]),
        ("PROD-A14", "LOC-TESLA", "CUST-TESLA", vec![15000.0, 16000.0, 18000.0, 22000.0, 25000.0, 20000.0, 17000.0, 15000.0]),
    ];
    for (prod, loc, cust, quantities) in demand_data {
        for (period, qty) in quantities.iter().enumerate() {
            let (d_type, firm) = if period < 2 { (DemandType::SalesOrder, true) } else { (DemandType::Forecast, false) };
            model.demands.push(Demand {
                id: Uuid::new_v4().to_string(), product_id: prod.into(), location_id: loc.into(),
                period: period as u32, quantity: *qty, priority: 1, non_delivery_cost: 50.0,
                late_delivery_cost: 5.0, demand_type: d_type, customer_id: Some(cust.into()), is_firm: firm,
            });
        }
    }

    let msg = format!(
        "Demo loaded: {} locations, {} products, {} resources, {} suppliers, {} customers, {} product-locations, {} demands, {} receipts, {} prices — {} periods",
        model.locations.len(), model.products.len(), model.resources.len(),
        model.suppliers.len(), model.customers.len(),
        model.product_locations.len(), model.demands.len(),
        model.planned_receipts.len(), model.product_prices.len(), model.num_periods
    );
    Json(msg)
}

// ═══════════════════════════════════════════════════════════════
// EXPLAINER
// ═══════════════════════════════════════════════════════════════

#[derive(Deserialize)]
struct ExplainPayload { question: String }

async fn explain_decision(AxState(state): AxState<AppState>, Json(p): Json<ExplainPayload>) -> impl IntoResponse {
    let model = state.model.lock().unwrap();
    let last = state.last_result.lock().unwrap();
    match last.as_ref() {
        Some(result) => Json(explainer::explain(&p.question, &model, result)),
        None => Json(explainer::ExplainResponse {
            intent: "error".into(),
            answer: "No optimization results available. Please run the optimizer first.".into(),
            data_points: vec![],
            suggestions: vec!["Go to Run Optimizer and click 🚀 Run Optimizer".into()],
        }),
    }
}

