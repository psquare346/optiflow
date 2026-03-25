// OptiFlow — Enhanced MILP Solver Engine
// Features: inventory balance, lead time offsets, BOM explosion,
//           calendar-based capacity, safety stock, holding costs,
//           time fences (frozen/firm/free), planned receipts,
//           max stock constraint, HashMap-based variable indexing.

use crate::models::*;
use highs::{HighsModelStatus, Model, RowProblem, Sense};
use std::collections::HashMap;
use std::time::Instant;

/// Main entry point — wraps build_and_solve with timing and error handling
pub fn solve(model: &SupplyChainModel, config: &SolverConfig) -> SolverResult {
    let start = Instant::now();
    match build_and_solve(model, config) {
        Ok(mut result) => {
            result.solve_time_ms = start.elapsed().as_millis() as u64;
            result
        }
        Err(e) => SolverResult {
            status: SolveStatus::Error(format!("Solver error: {}", e)),
            solve_time_ms: start.elapsed().as_millis() as u64,
            objective_value: 0.0,
            production_plan: vec![],
            transport_plan: vec![],
            inventory_plan: vec![],
            unmet_demand: vec![],
            capacity_utilization: vec![],
            kpis: DashboardKpis::default(),
        },
    }
}

// ─── Types for variable tracking ────────────────────────────

struct ProdVar { pid: String, lid: String, rid: String, period: u32, col: highs::Col, idx: usize }
struct TransVar { pid: String, from: String, to: String, period: u32, col: highs::Col, idx: usize }
struct InvVar { pid: String, lid: String, period: u32, col: highs::Col, idx: usize }
struct UnmetVar { demand_id: String, col: highs::Col, idx: usize }
struct SsViolVar { pid: String, lid: String, period: u32, col: highs::Col, idx: usize }
struct MaxStockViolVar { pid: String, lid: String, period: u32, col: highs::Col, idx: usize }

/// Determine which fence zone a period falls in
fn fence_zone(period: u32, config: &SolverConfig) -> FenceZone {
    if period < config.frozen_periods {
        FenceZone::Frozen
    } else if period < config.frozen_periods + config.firm_periods {
        FenceZone::Firm
    } else {
        FenceZone::Free
    }
}

// ─── Pre-computed lookup tables ─────────────────────────────
// Built once before solving, used O(1) throughout constraints.

struct Lookups {
    /// (product_id, location_id, period) → index into inv_vars
    inv_idx: HashMap<(String, String, u32), usize>,
    /// (product_id, location_id, period) → index into ss_viol_vars
    ss_idx: HashMap<(String, String, u32), usize>,
    /// (product_id, location_id, period) → index into max_stock_viol_vars
    ms_idx: HashMap<(String, String, u32), usize>,
    /// (from_location_id, to_location_id) → lead_time_periods
    lead_times: HashMap<(String, String), u32>,
    /// resource_id → Resource reference index
    resource_cost: HashMap<String, f64>,
    /// (from, to) → (cost_per_unit, tariff_rate)
    transport_costs: HashMap<(String, String), (f64, f64)>,
}

fn build_lookups(model: &SupplyChainModel) -> Lookups {
    let mut lead_times = HashMap::new();
    let mut transport_costs = HashMap::new();
    for tl in &model.transport_lanes {
        lead_times.insert((tl.from_location_id.clone(), tl.to_location_id.clone()), tl.lead_time_periods);
        transport_costs.insert(
            (tl.from_location_id.clone(), tl.to_location_id.clone()),
            (tl.cost_per_unit, tl.tariff_rate),
        );
    }
    let mut resource_cost = HashMap::new();
    for r in &model.resources {
        resource_cost.insert(r.id.clone(), r.cost_per_unit);
    }
    Lookups {
        inv_idx: HashMap::new(),
        ss_idx: HashMap::new(),
        ms_idx: HashMap::new(),
        lead_times,
        resource_cost,
        transport_costs,
    }
}

// ─── Build and Solve ────────────────────────────────────────

fn build_and_solve(
    model: &SupplyChainModel,
    config: &SolverConfig,
) -> Result<SolverResult, String> {
    let np = model.num_periods;
    let mut pb = RowProblem::default();
    let mut col_counter: usize = 0;
    let mut lookups = build_lookups(model);

    // ═══ DECISION VARIABLES ═══

    let mut prod_vars: Vec<ProdVar> = Vec::new();
    let mut trans_vars: Vec<TransVar> = Vec::new();
    let mut inv_vars: Vec<InvVar> = Vec::new();
    let mut unmet_vars: Vec<UnmetVar> = Vec::new();
    let mut ss_viol_vars: Vec<SsViolVar> = Vec::new();
    let mut ms_viol_vars: Vec<MaxStockViolVar> = Vec::new();

    // HashMap for fast lookup: (pid, lid, rid, period) → index in prod_vars
    let mut prod_var_idx: HashMap<(String, String, String, u32), usize> = HashMap::new();
    // (pid, from, to, period) → index in trans_vars
    let mut trans_var_idx: HashMap<(String, String, String, u32), usize> = HashMap::new();

    // ─── Production variables ───
    // In FROZEN zone: production is FIXED to planned receipt quantities (handled via constraint)
    // In FIRM zone: production allowed but with change penalty
    // In FREE zone: normal cost
    for sr in &model.sourcing_rules {
        if sr.sourcing_type != SourcingType::Production || !sr.active { continue; }
        let res_id = match &sr.resource_id {
            Some(r) => r,
            None => continue,
        };
        let base_cost = lookups.resource_cost.get(res_id)
            .copied()
            .unwrap_or(model.cost_profile.production_cost);

        for t in 0..np {
            let zone = fence_zone(t, config);
            let cost = match zone {
                FenceZone::Frozen => base_cost,  // will be fixed via constraint
                FenceZone::Firm => base_cost + config.firm_change_penalty,
                FenceZone::Free => base_cost,
            };

            let col = pb.add_column(cost, 0.0..);
            let idx = col_counter;
            col_counter += 1;
            prod_var_idx.insert(
                (sr.product_id.clone(), sr.from_location_id.clone(), res_id.clone(), t),
                prod_vars.len(),
            );
            prod_vars.push(ProdVar {
                pid: sr.product_id.clone(), lid: sr.from_location_id.clone(),
                rid: res_id.clone(), period: t, col, idx,
            });
        }
    }

    // ─── Transport variables ───
    for tl in &model.transport_lanes {
        if !tl.active { continue; }
        let products_on_lane: Vec<String> = model.sourcing_rules.iter()
            .filter(|sr| {
                sr.sourcing_type == SourcingType::Transport
                    && sr.from_location_id == tl.from_location_id
                    && sr.to_location_id.as_deref() == Some(&tl.to_location_id)
                    && sr.active
            })
            .map(|sr| sr.product_id.clone())
            .collect();

        for prod_id in products_on_lane {
            let (unit_cost, tariff) = lookups.transport_costs
                .get(&(tl.from_location_id.clone(), tl.to_location_id.clone()))
                .copied()
                .unwrap_or((model.cost_profile.transport_cost, 0.0));
            let base_cost = unit_cost * (1.0 + tariff);

            for t in 0..np {
                let zone = fence_zone(t, config);
                let cost = match zone {
                    FenceZone::Frozen => base_cost,
                    FenceZone::Firm => base_cost + config.firm_change_penalty,
                    FenceZone::Free => base_cost,
                };

                let col = pb.add_column(cost, 0.0..);
                let idx = col_counter;
                col_counter += 1;
                trans_var_idx.insert(
                    (prod_id.clone(), tl.from_location_id.clone(), tl.to_location_id.clone(), t),
                    trans_vars.len(),
                );
                trans_vars.push(TransVar {
                    pid: prod_id.clone(), from: tl.from_location_id.clone(),
                    to: tl.to_location_id.clone(), period: t, col, idx,
                });
            }
        }
    }

    // ─── Inventory variables ───
    for pl in &model.product_locations {
        if !pl.active { continue; }
        for t in 0..np {
            let col = pb.add_column(pl.holding_cost_per_unit, 0.0..);
            let idx = col_counter;
            col_counter += 1;
            lookups.inv_idx.insert((pl.product_id.clone(), pl.location_id.clone(), t), inv_vars.len());
            inv_vars.push(InvVar {
                pid: pl.product_id.clone(), lid: pl.location_id.clone(), period: t, col, idx,
            });
        }
    }

    // ─── Safety stock violation variables ───
    for pl in &model.product_locations {
        if !pl.active || pl.safety_stock <= 0.0 { continue; }
        for t in 0..np {
            let col = pb.add_column(pl.safety_stock_violation_cost, 0.0..);
            let idx = col_counter;
            col_counter += 1;
            lookups.ss_idx.insert((pl.product_id.clone(), pl.location_id.clone(), t), ss_viol_vars.len());
            ss_viol_vars.push(SsViolVar {
                pid: pl.product_id.clone(), lid: pl.location_id.clone(), period: t, col, idx,
            });
        }
    }

    // ─── Max stock violation variables (NEW) ───
    for pl in &model.product_locations {
        if !pl.active || pl.max_stock >= f64::MAX * 0.5 { continue; }
        for t in 0..np {
            let col = pb.add_column(pl.max_stock_violation_cost, 0.0..);
            let idx = col_counter;
            col_counter += 1;
            lookups.ms_idx.insert((pl.product_id.clone(), pl.location_id.clone(), t), ms_viol_vars.len());
            ms_viol_vars.push(MaxStockViolVar {
                pid: pl.product_id.clone(), lid: pl.location_id.clone(), period: t, col, idx,
            });
        }
    }

    // ─── Unmet demand variables ───
    let mut demand_unmet_idx: HashMap<String, usize> = HashMap::new();
    for demand in &model.demands {
        let col = pb.add_column(demand.non_delivery_cost, 0.0..);
        let idx = col_counter;
        col_counter += 1;
        demand_unmet_idx.insert(demand.id.clone(), unmet_vars.len());
        unmet_vars.push(UnmetVar { demand_id: demand.id.clone(), col, idx });
    }

    if prod_vars.is_empty() && trans_vars.is_empty() {
        return Err("No decision variables created. Check sourcing rules.".into());
    }

    // ═══ CONSTRAINTS ═══

    // 1. INVENTORY BALANCE per product × location × period
    //
    //    Inv[t] = Inv[t-1] + Production*yield + Inbound + PlannedReceipts
    //             - Outbound - Demand + Unmet - BOM_consumption
    //
    //    Rearranged (variables on LHS, constants on RHS):
    //    Inv[t] - Inv[t-1] - Prod*yield - Inbound + Outbound - Unmet + BOM
    //      = InitialInv(t=0) - Demand + PlannedReceipts
    //
    for pl in &model.product_locations {
        if !pl.active { continue; }
        let pid = &pl.product_id;
        let lid = &pl.location_id;
        let yield_rate = model.get_yield_rate(pid, lid);

        for t in 0..np {
            let mut row_factors: Vec<(highs::Col, f64)> = Vec::new();

            // +Inv[t]
            if let Some(&vi) = lookups.inv_idx.get(&(pid.clone(), lid.clone(), t)) {
                row_factors.push((inv_vars[vi].col, 1.0));
            }

            // -Inv[t-1]
            if t > 0 {
                if let Some(&vi) = lookups.inv_idx.get(&(pid.clone(), lid.clone(), t - 1)) {
                    row_factors.push((inv_vars[vi].col, -1.0));
                }
            }

            // -Production*yield (production ADDS to inventory)
            for pv in &prod_vars {
                if pv.pid == *pid && pv.lid == *lid && pv.period == t {
                    row_factors.push((pv.col, -yield_rate));
                }
            }

            // -Inbound transport with lead time
            for tv in &trans_vars {
                if tv.pid == *pid && tv.to == *lid {
                    let lead = lookups.lead_times.get(&(tv.from.clone(), tv.to.clone())).copied().unwrap_or(0);
                    if tv.period + lead == t {
                        row_factors.push((tv.col, -1.0));
                    }
                }
            }

            // +Outbound transport
            for tv in &trans_vars {
                if tv.pid == *pid && tv.from == *lid && tv.period == t {
                    row_factors.push((tv.col, 1.0));
                }
            }

            // -Unmet demand
            for demand in &model.demands {
                if demand.product_id == *pid && demand.location_id == *lid && demand.period == t {
                    if let Some(&ui) = demand_unmet_idx.get(&demand.id) {
                        row_factors.push((unmet_vars[ui].col, -1.0));
                    }
                }
            }

            // +BOM consumption
            for bom in &model.bom_entries {
                if bom.input_product_id == *pid
                    && (bom.location_id.is_none() || bom.location_id.as_deref() == Some(lid))
                {
                    for pv in &prod_vars {
                        if pv.pid == bom.output_product_id && pv.lid == *lid && pv.period == t {
                            row_factors.push((pv.col, bom.quantity_per));
                        }
                    }
                }
            }

            if row_factors.is_empty() { continue; }

            // RHS = initial_inventory(only t=0) - demand_at_t + planned_receipts_at_t
            let initial = if t == 0 { pl.initial_inventory } else { 0.0 };
            let demand_at_t: f64 = model.demands.iter()
                .filter(|d| d.product_id == *pid && d.location_id == *lid && d.period == t)
                .map(|d| d.quantity)
                .sum();
            // Planned receipts are known inflows (treated as constants on RHS)
            let receipts_at_t: f64 = model.planned_receipts.iter()
                .filter(|r| r.product_id == *pid && r.location_id == *lid && r.period == t)
                .map(|r| r.quantity)
                .sum();

            let rhs = initial - demand_at_t + receipts_at_t;
            pb.add_row(rhs..=rhs, row_factors);
        }
    }

    // 2. Demand satisfaction at locations WITHOUT ProductLocation record (backward compat)
    for demand in &model.demands {
        let has_pl = model.product_locations.iter().any(|pl| {
            pl.product_id == demand.product_id && pl.location_id == demand.location_id && pl.active
        });
        if has_pl { continue; }

        let mut row_factors: Vec<(highs::Col, f64)> = Vec::new();
        let yield_rate = model.get_yield_rate(&demand.product_id, &demand.location_id);

        for pv in &prod_vars {
            if pv.pid == demand.product_id && pv.lid == demand.location_id && pv.period == demand.period {
                row_factors.push((pv.col, yield_rate));
            }
        }
        for tv in &trans_vars {
            if tv.pid == demand.product_id && tv.to == demand.location_id {
                let lead = lookups.lead_times.get(&(tv.from.clone(), tv.to.clone())).copied().unwrap_or(0);
                if tv.period + lead == demand.period {
                    row_factors.push((tv.col, 1.0));
                }
            }
        }
        if let Some(&ui) = demand_unmet_idx.get(&demand.id) {
            row_factors.push((unmet_vars[ui].col, 1.0));
        }

        if !row_factors.is_empty() {
            pb.add_row(demand.quantity.., row_factors);
        }
    }

    // 3. CAPACITY CONSTRAINTS (with calendar support)
    for resource in &model.resources {
        if !resource.active { continue; }
        for t in 0..np {
            let effective_cap = model.effective_capacity(resource, t);
            if effective_cap <= 0.0 { continue; }

            let mut row_factors: Vec<(highs::Col, f64)> = Vec::new();
            for pv in &prod_vars {
                if pv.lid == resource.location_id && pv.rid == resource.id && pv.period == t {
                    let rate = model.get_consumption_rate(&pv.pid, &resource.id);
                    row_factors.push((pv.col, rate));
                }
            }
            if !row_factors.is_empty() {
                pb.add_row(..=effective_cap, row_factors);
            }
        }
    }

    // 4. SAFETY STOCK: Inv[t] + ss_violation[t] >= safety_stock
    for pl in &model.product_locations {
        if !pl.active || pl.safety_stock <= 0.0 { continue; }
        for t in 0..np {
            let mut row_factors: Vec<(highs::Col, f64)> = Vec::new();
            if let Some(&vi) = lookups.inv_idx.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                row_factors.push((inv_vars[vi].col, 1.0));
            }
            if let Some(&si) = lookups.ss_idx.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                row_factors.push((ss_viol_vars[si].col, 1.0));
            }
            if !row_factors.is_empty() {
                pb.add_row(pl.safety_stock.., row_factors);
            }
        }
    }

    // 5. MAX STOCK: Inv[t] - ms_violation[t] <= max_stock (NEW)
    for pl in &model.product_locations {
        if !pl.active || pl.max_stock >= f64::MAX * 0.5 { continue; }
        for t in 0..np {
            let mut row_factors: Vec<(highs::Col, f64)> = Vec::new();
            if let Some(&vi) = lookups.inv_idx.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                row_factors.push((inv_vars[vi].col, 1.0));
            }
            if let Some(&mi) = lookups.ms_idx.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                row_factors.push((ms_viol_vars[mi].col, -1.0));
            }
            if !row_factors.is_empty() {
                pb.add_row(..=pl.max_stock, row_factors);
            }
        }
    }

    // 6. FROZEN ZONE: Fix production and transport to zero (or planned receipts)
    //    In the frozen zone, the optimizer cannot create NEW supply decisions.
    //    Only planned_receipts (already on RHS) contribute to inventory.
    for pv in &prod_vars {
        if fence_zone(pv.period, config) == FenceZone::Frozen {
            // Fix production to 0 in frozen zone
            pb.add_row(0.0..=0.0, vec![(pv.col, 1.0)]);
        }
    }
    for tv in &trans_vars {
        if fence_zone(tv.period, config) == FenceZone::Frozen {
            // Fix transport to 0 in frozen zone
            pb.add_row(0.0..=0.0, vec![(tv.col, 1.0)]);
        }
    }

    // ═══ SOLVE ═══
    let mut highs_model = Model::new(pb);
    highs_model.set_option("time_limit", config.time_limit_seconds);
    highs_model.set_option("mip_rel_gap", config.optimality_gap);
    highs_model.set_sense(Sense::Minimise);

    let solved = highs_model.solve();

    let status = match solved.status() {
        HighsModelStatus::Optimal => SolveStatus::Optimal,
        HighsModelStatus::Infeasible => SolveStatus::Infeasible,
        HighsModelStatus::ObjectiveBound => SolveStatus::Feasible,
        _ => SolveStatus::Error(format!("Solver status: {:?}", solved.status())),
    };

    if status == SolveStatus::Infeasible {
        return Ok(SolverResult {
            status, solve_time_ms: 0, objective_value: 0.0,
            production_plan: vec![], transport_plan: vec![], inventory_plan: vec![],
            unmet_demand: vec![], capacity_utilization: vec![], kpis: DashboardKpis::default(),
        });
    }

    // ═══ EXTRACT RESULTS ═══
    let solution = solved.get_solution();
    let col_values = solution.columns();

    // Production plan
    let mut production_plan = Vec::new();
    let mut total_production_cost = 0.0;
    for pv in &prod_vars {
        let qty = col_values[pv.idx];
        if qty > 0.001 {
            let base_cost = lookups.resource_cost.get(&pv.rid).copied().unwrap_or(model.cost_profile.production_cost);
            let cost = qty * base_cost;
            total_production_cost += cost;
            production_plan.push(PlanEntry {
                product_id: pv.pid.clone(), location_id: pv.lid.clone(), resource_id: pv.rid.clone(),
                period: pv.period, quantity: qty, cost,
            });
        }
    }

    // Transport plan
    let mut transport_plan = Vec::new();
    let mut total_transport_cost = 0.0;
    for tv in &trans_vars {
        let qty = col_values[tv.idx];
        if qty > 0.001 {
            let (unit_cost, tariff) = lookups.transport_costs
                .get(&(tv.from.clone(), tv.to.clone()))
                .copied()
                .unwrap_or((model.cost_profile.transport_cost, 0.0));
            let cost = qty * unit_cost * (1.0 + tariff);
            total_transport_cost += cost;
            let mode = model.transport_lanes.iter()
                .find(|tl| tl.from_location_id == tv.from && tl.to_location_id == tv.to)
                .map(|tl| tl.mode.clone())
                .unwrap_or(TransportMode::Truck);
            transport_plan.push(TransportPlanEntry {
                product_id: tv.pid.clone(), from_location_id: tv.from.clone(),
                to_location_id: tv.to.clone(), period: tv.period, quantity: qty, cost, mode,
            });
        }
    }

    // Inventory plan
    let mut inventory_plan = Vec::new();
    let mut total_holding_cost = 0.0;
    let mut all_inv_values: Vec<f64> = Vec::new();
    for iv in &inv_vars {
        let qty = col_values[iv.idx];
        let pl = model.get_product_location(&iv.pid, &iv.lid);
        let holding = pl.map(|p| p.holding_cost_per_unit).unwrap_or(0.5);
        let hcost = qty * holding;
        total_holding_cost += hcost;
        all_inv_values.push(qty);

        let ss = pl.map(|p| p.safety_stock).unwrap_or(0.0);
        inventory_plan.push(InventoryEntry {
            product_id: iv.pid.clone(), location_id: iv.lid.clone(), period: iv.period,
            quantity: qty, holding_cost: hcost, safety_stock_delta: qty - ss,
        });
    }

    // Unmet demand
    let mut unmet_demand = Vec::new();
    let mut total_penalty_cost = 0.0;
    for uv in &unmet_vars {
        let qty = col_values[uv.idx];
        if qty > 0.001 {
            if let Some(demand) = model.demands.iter().find(|d| d.id == uv.demand_id) {
                let penalty = qty * demand.non_delivery_cost;
                total_penalty_cost += penalty;
                unmet_demand.push(UnmetDemandEntry {
                    demand_id: uv.demand_id.clone(), product_id: demand.product_id.clone(),
                    location_id: demand.location_id.clone(), period: demand.period,
                    unmet_quantity: qty, penalty_cost: penalty,
                    reason: "Insufficient capacity or too costly to deliver".into(),
                });
            }
        }
    }

    // Safety stock + max stock violation costs
    for sv in &ss_viol_vars {
        let qty = col_values[sv.idx];
        if qty > 0.001 { total_penalty_cost += qty; }
    }
    for mv in &ms_viol_vars {
        let qty = col_values[mv.idx];
        if qty > 0.001 { total_penalty_cost += qty; }
    }

    // Capacity utilization
    let mut capacity_utilization = Vec::new();
    for resource in &model.resources {
        if !resource.active { continue; }
        for t in 0..np {
            let effective_cap = model.effective_capacity(resource, t);
            let mut used = 0.0;
            for pv in &prod_vars {
                if pv.lid == resource.location_id && pv.rid == resource.id && pv.period == t {
                    let rate = model.get_consumption_rate(&pv.pid, &resource.id);
                    used += col_values[pv.idx] * rate;
                }
            }
            let util_pct = if effective_cap > 0.0 { (used / effective_cap * 100.0).min(100.0) } else { 0.0 };
            capacity_utilization.push(CapacityUtilEntry {
                resource_id: resource.id.clone(), location_id: resource.location_id.clone(),
                period: t, used, available: effective_cap, utilization_pct: util_pct,
            });
        }
    }

    // ═══ KPIs ═══
    let total_demand_qty: f64 = model.demands.iter().map(|d| d.quantity).sum();
    let total_unmet_qty: f64 = unmet_demand.iter().map(|u| u.unmet_quantity).sum();
    let total_delivered = total_demand_qty - total_unmet_qty;
    let total_cost = total_production_cost + total_transport_cost + total_holding_cost + total_penalty_cost;

    let avg_util = if capacity_utilization.is_empty() { 0.0 }
        else { capacity_utilization.iter().map(|c| c.utilization_pct).sum::<f64>() / capacity_utilization.len() as f64 };
    let bottlenecks = capacity_utilization.iter().filter(|c| c.utilization_pct > 95.0).count() as u32;
    let avg_inv = if all_inv_values.is_empty() { 0.0 }
        else { all_inv_values.iter().sum::<f64>() / all_inv_values.len() as f64 };
    let peak_inv = all_inv_values.iter().cloned().fold(0.0_f64, f64::max);

    let kpis = DashboardKpis {
        demand_fulfillment_pct: if total_demand_qty > 0.0 { (total_delivered / total_demand_qty * 100.0).min(100.0) } else { 100.0 },
        total_delivered, total_demand: total_demand_qty, total_unmet: total_unmet_qty,
        total_cost, production_cost: total_production_cost, transport_cost: total_transport_cost,
        holding_cost: total_holding_cost, penalty_cost: total_penalty_cost,
        cost_per_unit_delivered: if total_delivered > 0.0 { total_cost / total_delivered } else { 0.0 },
        avg_capacity_utilization: avg_util, num_bottleneck_resources: bottlenecks,
        avg_inventory: avg_inv, peak_inventory: peak_inv,
    };

    Ok(SolverResult {
        status, solve_time_ms: 0, objective_value: solved.objective_value(),
        production_plan, transport_plan, inventory_plan, unmet_demand, capacity_utilization, kpis,
    })
}
