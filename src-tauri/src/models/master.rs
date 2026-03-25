// OptiFlow — Master data types (relatively static entities)

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::enums::*;

// ─── Location ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub location_type: LocationType,
    pub country: String,
    pub region: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub active: bool,
}

impl Location {
    pub fn new(id: &str, name: &str, loc_type: LocationType, country: &str, region: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            location_type: loc_type,
            country: country.to_string(),
            region: region.to_string(),
            latitude: None,
            longitude: None,
            active: true,
        }
    }
}

// ─── Product ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub product_type: ProductType,
    pub unit_of_measure: String,
    pub yield_rate: f64,
    pub weight_kg: f64,
    pub volume_m3: f64,
    pub shelf_life_days: Option<u32>,
    pub active: bool,
}

impl Product {
    pub fn new(id: &str, name: &str, ptype: ProductType, uom: &str, yield_rate: f64) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            product_type: ptype,
            unit_of_measure: uom.to_string(),
            yield_rate: yield_rate.clamp(0.01, 1.0),
            weight_kg: 0.0,
            volume_m3: 0.0,
            shelf_life_days: None,
            active: true,
        }
    }
}

// ─── Resource (capacity constraint) ─────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub capacity_type: CapacityType,
    pub capacity_per_period: f64,
    pub cost_per_unit: f64,
    pub setup_cost: f64,
    pub active: bool,
}

impl Resource {
    pub fn new(id: &str, name: &str, location: &str, cap: f64, cost: f64) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            location_id: location.to_string(),
            capacity_type: CapacityType::Units,
            capacity_per_period: cap,
            cost_per_unit: cost,
            setup_cost: 0.0,
            active: true,
        }
    }
}

// ─── Transport Lane ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportLane {
    pub id: String,
    pub from_location_id: String,
    pub to_location_id: String,
    pub mode: TransportMode,
    pub cost_per_unit: f64,
    pub fixed_cost_per_shipment: f64,
    pub lead_time_periods: u32,
    pub lead_time_days: u32,
    pub min_lot_size: f64,
    pub max_lot_size: f64,
    pub tariff_rate: f64,
    pub co2_per_unit: f64,
    pub active: bool,
}

impl TransportLane {
    pub fn new(from_loc: &str, to_loc: &str, mode: TransportMode, cost: f64, lead_time_days: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_location_id: from_loc.to_string(),
            to_location_id: to_loc.to_string(),
            mode,
            cost_per_unit: cost,
            fixed_cost_per_shipment: 0.0,
            lead_time_periods: 0,
            lead_time_days,
            min_lot_size: 0.0,
            max_lot_size: f64::MAX,
            tariff_rate: 0.0,
            co2_per_unit: 0.0,
            active: true,
        }
    }
}

// ─── Supplier (NEW) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub country: String,
    pub lead_time_days: u32,
    pub capacity_per_period: f64,
    pub quality_rating: f64,
    pub active: bool,
}

impl Supplier {
    pub fn new(id: &str, name: &str, country: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            country: country.to_string(),
            lead_time_days: 7,
            capacity_per_period: f64::MAX,
            quality_rating: 1.0,
            active: true,
        }
    }
}

// ─── Customer (NEW) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub priority: u32,
    pub service_level_target: f64,
    pub country: String,
    pub active: bool,
}

impl Customer {
    pub fn new(id: &str, name: &str, country: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            priority: 1,
            service_level_target: 0.95,
            country: country.to_string(),
            active: true,
        }
    }
}
