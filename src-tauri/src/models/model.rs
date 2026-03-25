// OptiFlow — Supply Chain Model (the main container)
// Holds all master data, transactions, and provides helper methods.

use serde::{Deserialize, Serialize};
use super::master::*;
use super::relationships::*;
use super::transactions::*;
use super::calendar::*;

// ─── Cost Profile ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostProfile {
    pub production_cost: f64,
    pub fixed_production_cost: f64,
    pub transport_cost: f64,
    pub procurement_cost: f64,
    pub holding_cost: f64,
    pub safety_stock_violation_cost: f64,
    pub max_stock_violation_cost: f64,
}

impl Default for CostProfile {
    fn default() -> Self {
        Self {
            production_cost: 10.0,
            fixed_production_cost: 0.0,
            transport_cost: 1.0,
            procurement_cost: 5.0,
            holding_cost: 0.5,
            safety_stock_violation_cost: 10.0,
            max_stock_violation_cost: 10.0,
        }
    }
}

// ─── Supply Chain Model ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainModel {
    pub name: String,
    pub num_periods: u32,

    // Master data
    pub locations: Vec<Location>,
    pub products: Vec<Product>,
    pub resources: Vec<Resource>,
    pub transport_lanes: Vec<TransportLane>,
    pub suppliers: Vec<Supplier>,
    pub customers: Vec<Customer>,

    // Relationships
    pub bom_entries: Vec<BomEntry>,
    pub sourcing_rules: Vec<SourcingRule>,
    pub product_locations: Vec<ProductLocation>,
    pub product_resources: Vec<ProductResource>,
    pub customer_quotas: Vec<CustomerQuota>,

    // Transactions
    pub demands: Vec<Demand>,
    pub planned_receipts: Vec<PlannedReceipt>,
    pub product_prices: Vec<ProductPrice>,

    // Calendar
    pub calendar_entries: Vec<CalendarEntry>,
    pub time_buckets: Vec<TimeBucket>,
    pub planning_calendar: Option<PlanningCalendar>,

    pub cost_profile: CostProfile,
}

impl SupplyChainModel {
    pub fn new(name: &str, num_periods: u32) -> Self {
        Self {
            name: name.to_string(),
            num_periods,
            locations: Vec::new(),
            products: Vec::new(),
            resources: Vec::new(),
            transport_lanes: Vec::new(),
            suppliers: Vec::new(),
            customers: Vec::new(),
            bom_entries: Vec::new(),
            sourcing_rules: Vec::new(),
            product_locations: Vec::new(),
            product_resources: Vec::new(),
            customer_quotas: Vec::new(),
            demands: Vec::new(),
            planned_receipts: Vec::new(),
            product_prices: Vec::new(),
            calendar_entries: Vec::new(),
            time_buckets: Vec::new(),
            planning_calendar: None,
            cost_profile: CostProfile::default(),
        }
    }

    pub fn total_demand(&self) -> f64 {
        self.demands.iter().map(|d| d.quantity).sum()
    }

    /// Get effective capacity for a resource at a given period,
    /// accounting for calendar overrides.
    pub fn effective_capacity(&self, resource: &Resource, period: u32) -> f64 {
        // Check resource-specific calendar entry
        if let Some(cal) = self.calendar_entries.iter().find(|c| {
            c.resource_id.as_deref() == Some(&resource.id)
                && c.location_id == resource.location_id
                && c.period == period
        }) {
            if !cal.is_working {
                return 0.0;
            }
            if let Some(cap_override) = cal.available_capacity {
                return cap_override * cal.shift_factor;
            }
            return resource.capacity_per_period * cal.shift_factor;
        }

        // Check location-level calendar entry
        if let Some(cal) = self.calendar_entries.iter().find(|c| {
            c.resource_id.is_none()
                && c.location_id == resource.location_id
                && c.period == period
        }) {
            if !cal.is_working {
                return 0.0;
            }
            return resource.capacity_per_period * cal.shift_factor;
        }

        resource.capacity_per_period
    }

    /// Get ProductLocation record, or None if not defined.
    pub fn get_product_location(&self, product_id: &str, location_id: &str) -> Option<&ProductLocation> {
        self.product_locations.iter().find(|pl| {
            pl.product_id == product_id && pl.location_id == location_id && pl.active
        })
    }

    /// Get yield rate (location override > product default > 1.0)
    pub fn get_yield_rate(&self, product_id: &str, location_id: &str) -> f64 {
        if let Some(pl) = self.get_product_location(product_id, location_id) {
            if let Some(yr) = pl.yield_rate_override {
                return yr;
            }
        }
        self.products.iter().find(|p| p.id == product_id)
            .map(|p| p.yield_rate)
            .unwrap_or(1.0)
    }

    /// Get resource consumption rate for a product on a resource
    pub fn get_consumption_rate(&self, product_id: &str, resource_id: &str) -> f64 {
        self.product_resources.iter()
            .find(|pr| pr.product_id == product_id && pr.resource_id == resource_id && pr.active)
            .map(|pr| pr.consumption_rate)
            .unwrap_or(1.0)
    }

    /// Get selling price for a product (for MaxProfit objective)
    pub fn get_price(&self, product_id: &str, location_id: &str, period: u32) -> f64 {
        // Most specific match: product + location + period
        if let Some(pp) = self.product_prices.iter().find(|pp| {
            pp.product_id == product_id
                && pp.location_id.as_deref() == Some(location_id)
                && pp.period == Some(period)
        }) {
            return pp.price_per_unit;
        }
        // Product + location (any period)
        if let Some(pp) = self.product_prices.iter().find(|pp| {
            pp.product_id == product_id
                && pp.location_id.as_deref() == Some(location_id)
                && pp.period.is_none()
        }) {
            return pp.price_per_unit;
        }
        // Product only (global price)
        if let Some(pp) = self.product_prices.iter().find(|pp| {
            pp.product_id == product_id
                && pp.location_id.is_none()
                && pp.period.is_none()
        }) {
            return pp.price_per_unit;
        }
        0.0
    }
}
