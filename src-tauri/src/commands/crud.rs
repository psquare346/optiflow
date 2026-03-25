// OptiFlow — CRUD command handlers for all entity types
// Uses a consistent pattern: get_*, add_*, delete_* for each collection.

use crate::models::*;
use crate::state::AppState;
use tauri::State;

// ═══════════════════════════════════════════════════════════════
// LOCATION CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_locations(state: State<AppState>) -> Vec<Location> {
    state.model.lock().unwrap().locations.clone()
}

#[tauri::command]
pub fn add_location(state: State<AppState>, location: Location) {
    state.model.lock().unwrap().locations.push(location);
}

#[tauri::command]
pub fn delete_location(state: State<AppState>, id: String) {
    state.model.lock().unwrap().locations.retain(|l| l.id != id);
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_products(state: State<AppState>) -> Vec<Product> {
    state.model.lock().unwrap().products.clone()
}

#[tauri::command]
pub fn add_product(state: State<AppState>, product: Product) {
    state.model.lock().unwrap().products.push(product);
}

#[tauri::command]
pub fn delete_product(state: State<AppState>, id: String) {
    state.model.lock().unwrap().products.retain(|p| p.id != id);
}

// ═══════════════════════════════════════════════════════════════
// RESOURCE CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_resources(state: State<AppState>) -> Vec<Resource> {
    state.model.lock().unwrap().resources.clone()
}

#[tauri::command]
pub fn add_resource(state: State<AppState>, resource: Resource) {
    state.model.lock().unwrap().resources.push(resource);
}

#[tauri::command]
pub fn delete_resource(state: State<AppState>, id: String) {
    state.model.lock().unwrap().resources.retain(|r| r.id != id);
}

// ═══════════════════════════════════════════════════════════════
// TRANSPORT LANE CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_transport_lanes(state: State<AppState>) -> Vec<TransportLane> {
    state.model.lock().unwrap().transport_lanes.clone()
}

#[tauri::command]
pub fn add_transport_lane(state: State<AppState>, lane: TransportLane) {
    state.model.lock().unwrap().transport_lanes.push(lane);
}

#[tauri::command]
pub fn delete_transport_lane(state: State<AppState>, id: String) {
    state.model.lock().unwrap().transport_lanes.retain(|t| t.id != id);
}

// ═══════════════════════════════════════════════════════════════
// SUPPLIER CRUD (NEW)
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_suppliers(state: State<AppState>) -> Vec<Supplier> {
    state.model.lock().unwrap().suppliers.clone()
}

#[tauri::command]
pub fn add_supplier(state: State<AppState>, supplier: Supplier) {
    state.model.lock().unwrap().suppliers.push(supplier);
}

#[tauri::command]
pub fn delete_supplier(state: State<AppState>, id: String) {
    state.model.lock().unwrap().suppliers.retain(|s| s.id != id);
}

// ═══════════════════════════════════════════════════════════════
// CUSTOMER CRUD (NEW)
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_customers(state: State<AppState>) -> Vec<Customer> {
    state.model.lock().unwrap().customers.clone()
}

#[tauri::command]
pub fn add_customer(state: State<AppState>, customer: Customer) {
    state.model.lock().unwrap().customers.push(customer);
}

#[tauri::command]
pub fn delete_customer(state: State<AppState>, id: String) {
    state.model.lock().unwrap().customers.retain(|c| c.id != id);
}

// ═══════════════════════════════════════════════════════════════
// DEMAND CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_demands(state: State<AppState>) -> Vec<Demand> {
    state.model.lock().unwrap().demands.clone()
}

#[tauri::command]
pub fn add_demand(state: State<AppState>, demand: Demand) {
    state.model.lock().unwrap().demands.push(demand);
}

#[tauri::command]
pub fn delete_demand(state: State<AppState>, id: String) {
    state.model.lock().unwrap().demands.retain(|d| d.id != id);
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT-LOCATION CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_product_locations(state: State<AppState>) -> Vec<ProductLocation> {
    state.model.lock().unwrap().product_locations.clone()
}

#[tauri::command]
pub fn add_product_location(state: State<AppState>, pl: ProductLocation) {
    state.model.lock().unwrap().product_locations.push(pl);
}

#[tauri::command]
pub fn delete_product_location(state: State<AppState>, product_id: String, location_id: String) {
    state.model.lock().unwrap().product_locations.retain(|pl| {
        !(pl.product_id == product_id && pl.location_id == location_id)
    });
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT-RESOURCE CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_product_resources(state: State<AppState>) -> Vec<ProductResource> {
    state.model.lock().unwrap().product_resources.clone()
}

#[tauri::command]
pub fn add_product_resource(state: State<AppState>, pr: ProductResource) {
    state.model.lock().unwrap().product_resources.push(pr);
}

#[tauri::command]
pub fn delete_product_resource(state: State<AppState>, product_id: String, resource_id: String) {
    state.model.lock().unwrap().product_resources.retain(|pr| {
        !(pr.product_id == product_id && pr.resource_id == resource_id)
    });
}

// ═══════════════════════════════════════════════════════════════
// BOM CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_bom_entries(state: State<AppState>) -> Vec<BomEntry> {
    state.model.lock().unwrap().bom_entries.clone()
}

#[tauri::command]
pub fn add_bom_entry(state: State<AppState>, entry: BomEntry) {
    state.model.lock().unwrap().bom_entries.push(entry);
}

// ═══════════════════════════════════════════════════════════════
// SOURCING RULE CRUD
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_sourcing_rules(state: State<AppState>) -> Vec<SourcingRule> {
    state.model.lock().unwrap().sourcing_rules.clone()
}

#[tauri::command]
pub fn add_sourcing_rule(state: State<AppState>, rule: SourcingRule) {
    state.model.lock().unwrap().sourcing_rules.push(rule);
}

// ═══════════════════════════════════════════════════════════════
// PLANNED RECEIPT CRUD (NEW)
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_planned_receipts(state: State<AppState>) -> Vec<PlannedReceipt> {
    state.model.lock().unwrap().planned_receipts.clone()
}

#[tauri::command]
pub fn add_planned_receipt(state: State<AppState>, receipt: PlannedReceipt) {
    state.model.lock().unwrap().planned_receipts.push(receipt);
}

#[tauri::command]
pub fn delete_planned_receipt(state: State<AppState>, id: String) {
    state.model.lock().unwrap().planned_receipts.retain(|r| r.id != id);
}

// ═══════════════════════════════════════════════════════════════
// PRODUCT PRICE CRUD (NEW)
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_product_prices(state: State<AppState>) -> Vec<ProductPrice> {
    state.model.lock().unwrap().product_prices.clone()
}

#[tauri::command]
pub fn add_product_price(state: State<AppState>, price: ProductPrice) {
    state.model.lock().unwrap().product_prices.push(price);
}

// ═══════════════════════════════════════════════════════════════
// ALIAS SYSTEM
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_aliases(state: State<AppState>) -> AliasMap {
    state.aliases.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_alias(state: State<AppState>, internal_name: String, display_name: String) {
    state.aliases.lock().unwrap().set_alias(&internal_name, &display_name);
}
