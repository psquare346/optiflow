// OptiFlow — Relationship / compound tables
// These define HOW entities interact (product↔location, product↔resource, BOM, sourcing)

use serde::{Deserialize, Serialize};
use super::enums::*;

// ─── Product-Location (most critical intersection) ──────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductLocation {
    pub product_id: String,
    pub location_id: String,

    // Inventory params
    pub initial_inventory: f64,
    pub safety_stock: f64,
    pub max_stock: f64,
    pub holding_cost_per_unit: f64,
    pub safety_stock_violation_cost: f64,
    pub max_stock_violation_cost: f64,
    pub shelf_life_periods: Option<u32>,

    // Production params
    pub can_produce: bool,
    pub yield_rate_override: Option<f64>,
    pub production_lead_time: u32,

    // Procurement params
    pub can_procure: bool,
    pub procurement_cost: f64,
    pub procurement_lead_time: u32,
    pub min_procurement_qty: f64,

    // Lot sizing
    pub min_lot_size: f64,
    pub max_lot_size: f64,
    pub lot_rounding: f64,

    // Flags
    pub can_store: bool,
    pub active: bool,
}

impl ProductLocation {
    pub fn new(product_id: &str, location_id: &str) -> Self {
        Self {
            product_id: product_id.to_string(),
            location_id: location_id.to_string(),
            initial_inventory: 0.0,
            safety_stock: 0.0,
            max_stock: f64::MAX,
            holding_cost_per_unit: 0.5,
            safety_stock_violation_cost: 10.0,
            max_stock_violation_cost: 10.0,
            shelf_life_periods: None,
            can_produce: false,
            yield_rate_override: None,
            production_lead_time: 0,
            can_procure: false,
            procurement_cost: 0.0,
            procurement_lead_time: 0,
            min_procurement_qty: 0.0,
            min_lot_size: 0.0,
            max_lot_size: f64::MAX,
            lot_rounding: 1.0,
            can_store: true,
            active: true,
        }
    }
}

// ─── Product-Resource (Production Data Structure) ───────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResource {
    pub product_id: String,
    pub resource_id: String,
    pub location_id: String,
    pub consumption_rate: f64,
    pub setup_time: f64,
    pub changeover_cost: f64,
    pub production_rate: f64,
    pub version_id: String,
    pub priority: u32,
    pub active: bool,
}

impl ProductResource {
    pub fn new(product_id: &str, resource_id: &str, location_id: &str) -> Self {
        Self {
            product_id: product_id.to_string(),
            resource_id: resource_id.to_string(),
            location_id: location_id.to_string(),
            consumption_rate: 1.0,
            setup_time: 0.0,
            changeover_cost: 0.0,
            production_rate: f64::MAX,
            version_id: "V1".to_string(),
            priority: 1,
            active: true,
        }
    }
}

// ─── BOM (Bill of Materials) ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomEntry {
    pub output_product_id: String,
    pub input_product_id: String,
    pub quantity_per: f64,
    pub yield_rate: f64,
    pub location_id: Option<String>,
}

// ─── Sourcing Rule ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcingRule {
    pub id: String,
    pub sourcing_type: SourcingType,
    pub product_id: String,
    pub from_location_id: String,
    pub to_location_id: Option<String>,
    pub resource_id: Option<String>,
    pub priority: u32,
    pub quota_percentage: f64,
    pub min_lot_size: f64,
    pub max_lot_size: f64,
    pub active: bool,
}

// ─── Customer Product Quota (fair-share allocation) ─────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerQuota {
    pub product_id: String,
    pub customer_location_id: String,
    pub period: Option<u32>,
    pub max_share_pct: f64,
    pub min_share_pct: f64,
    pub priority_group: u32,
}
