// OptiFlow — Solver configuration, result types, and KPIs

use serde::{Deserialize, Serialize};
use super::enums::*;

// ─── Solver Configuration ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    pub objective: Objective,
    pub time_limit_seconds: f64,
    pub optimality_gap: f64,
    /// Number of frozen periods (optimizer cannot change decisions)
    pub frozen_periods: u32,
    /// Number of firm periods (hard constraints, change penalty)
    pub firm_periods: u32,
    /// Cost penalty for changing decisions in the firm zone
    pub firm_change_penalty: f64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            objective: Objective::MinimizeCost,
            time_limit_seconds: 300.0,
            optimality_gap: 0.01,
            frozen_periods: 0,
            firm_periods: 0,
            firm_change_penalty: 100.0,
        }
    }
}

// ─── Solver Result ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverResult {
    pub status: SolveStatus,
    pub solve_time_ms: u64,
    pub objective_value: f64,
    pub production_plan: Vec<PlanEntry>,
    pub transport_plan: Vec<TransportPlanEntry>,
    pub inventory_plan: Vec<InventoryEntry>,
    pub unmet_demand: Vec<UnmetDemandEntry>,
    pub capacity_utilization: Vec<CapacityUtilEntry>,
    pub kpis: DashboardKpis,
}

// ─── Plan Entry (production) ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEntry {
    pub product_id: String,
    pub location_id: String,
    pub resource_id: String,
    pub period: u32,
    pub quantity: f64,
    pub cost: f64,
}

// ─── Transport Plan Entry ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportPlanEntry {
    pub product_id: String,
    pub from_location_id: String,
    pub to_location_id: String,
    pub period: u32,
    pub quantity: f64,
    pub cost: f64,
    pub mode: TransportMode,
}

// ─── Inventory Entry ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryEntry {
    pub product_id: String,
    pub location_id: String,
    pub period: u32,
    pub quantity: f64,
    pub holding_cost: f64,
    pub safety_stock_delta: f64,
}

// ─── Unmet Demand Entry ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmetDemandEntry {
    pub demand_id: String,
    pub product_id: String,
    pub location_id: String,
    pub period: u32,
    pub unmet_quantity: f64,
    pub penalty_cost: f64,
    pub reason: String,
}

// ─── Capacity Utilization ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityUtilEntry {
    pub resource_id: String,
    pub location_id: String,
    pub period: u32,
    pub used: f64,
    pub available: f64,
    pub utilization_pct: f64,
}

// ─── KPI Dashboard ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardKpis {
    pub demand_fulfillment_pct: f64,
    pub total_delivered: f64,
    pub total_demand: f64,
    pub total_unmet: f64,
    pub total_cost: f64,
    pub production_cost: f64,
    pub transport_cost: f64,
    pub holding_cost: f64,
    pub penalty_cost: f64,
    pub cost_per_unit_delivered: f64,
    pub avg_capacity_utilization: f64,
    pub num_bottleneck_resources: u32,
    pub avg_inventory: f64,
    pub peak_inventory: f64,
}

impl Default for DashboardKpis {
    fn default() -> Self {
        Self {
            demand_fulfillment_pct: 0.0,
            total_delivered: 0.0,
            total_demand: 0.0,
            total_unmet: 0.0,
            total_cost: 0.0,
            production_cost: 0.0,
            transport_cost: 0.0,
            holding_cost: 0.0,
            penalty_cost: 0.0,
            cost_per_unit_delivered: 0.0,
            avg_capacity_utilization: 0.0,
            num_bottleneck_resources: 0,
            avg_inventory: 0.0,
            peak_inventory: 0.0,
        }
    }
}

// ─── Validation ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    pub severity: ValidationSeverity,
    pub category: String,
    pub message: String,
    pub field: Option<String>,
    pub suggestion: Option<String>,
}

// ─── Alias System ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasMap {
    pub aliases: std::collections::HashMap<String, String>,
}

impl Default for AliasMap {
    fn default() -> Self {
        let mut aliases = std::collections::HashMap::new();
        aliases.insert("location_id".into(), "Location ID".into());
        aliases.insert("location_type".into(), "Type".into());
        aliases.insert("product_id".into(), "Product ID".into());
        aliases.insert("product_type".into(), "Type".into());
        aliases.insert("yield_rate".into(), "Yield %".into());
        aliases.insert("capacity_per_period".into(), "Capacity / Period".into());
        aliases.insert("cost_per_unit".into(), "Unit Cost".into());
        aliases.insert("lead_time_days".into(), "Lead Time (days)".into());
        aliases.insert("non_delivery_cost".into(), "Non-Delivery Penalty".into());
        aliases.insert("late_delivery_cost".into(), "Late Penalty".into());
        aliases.insert("tariff_rate".into(), "Tariff %".into());
        aliases.insert("initial_inventory".into(), "Opening Stock".into());
        aliases.insert("safety_stock".into(), "Safety Stock".into());
        aliases.insert("max_stock".into(), "Max Stock".into());
        aliases.insert("holding_cost_per_unit".into(), "Holding Cost".into());
        aliases.insert("consumption_rate".into(), "Resource Consumption".into());
        aliases.insert("demand_fulfillment_pct".into(), "Demand Fill Rate".into());
        aliases.insert("total_cost".into(), "Total Plan Cost".into());
        aliases.insert("production_cost".into(), "Manufacturing Cost".into());
        aliases.insert("transport_cost".into(), "Logistics Cost".into());
        aliases.insert("holding_cost".into(), "Inventory Holding Cost".into());
        aliases.insert("penalty_cost".into(), "Penalty Exposure".into());
        aliases.insert("cost_per_unit_delivered".into(), "Cost to Serve".into());
        aliases.insert("avg_capacity_utilization".into(), "Avg Plant Loading".into());
        aliases.insert("num_bottleneck_resources".into(), "Bottlenecks".into());
        aliases.insert("avg_inventory".into(), "Avg Stock Level".into());
        aliases.insert("total_delivered".into(), "Units Delivered".into());
        aliases.insert("total_unmet".into(), "Shortfall".into());
        Self { aliases }
    }
}

impl AliasMap {
    pub fn get_label(&self, internal_name: &str) -> String {
        self.aliases
            .get(internal_name)
            .cloned()
            .unwrap_or_else(|| internal_name.replace('_', " "))
    }

    pub fn set_alias(&mut self, internal_name: &str, display_name: &str) {
        self.aliases
            .insert(internal_name.to_string(), display_name.to_string());
    }
}
