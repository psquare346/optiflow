// OptiFlow — Transaction data types (time-phased, frequently changing)

use serde::{Deserialize, Serialize};
use super::enums::*;

// ─── Demand ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demand {
    pub id: String,
    pub product_id: String,
    pub location_id: String,
    pub period: u32,
    pub quantity: f64,
    pub priority: u32,
    pub non_delivery_cost: f64,
    pub late_delivery_cost: f64,
    pub demand_type: DemandType,
    pub customer_id: Option<String>,
    pub is_firm: bool,
}

// ─── Planned Receipt (open POs, production orders, in-transit) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedReceipt {
    pub id: String,
    pub product_id: String,
    pub location_id: String,
    pub period: u32,
    pub quantity: f64,
    pub receipt_type: ReceiptType,
    pub source: Option<String>,
    pub is_firm: bool,
}

// ─── Product Price (revenue for MaxProfit objective) ────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPrice {
    pub product_id: String,
    pub location_id: Option<String>,
    pub customer_id: Option<String>,
    pub period: Option<u32>,
    pub price_per_unit: f64,
}
