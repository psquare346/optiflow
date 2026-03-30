// OptiFlow — MILP Solver Engine (Mixed-Integer Linear Programming)
// Features: inventory balance, lead time offsets, BOM explosion,
//           calendar-based capacity, safety stock, holding costs,
//           time fences (frozen/firm/free), planned receipts,
//           max stock constraint, setup costs (binary), fixed shipment
//           costs (binary), min/max lot sizing via big-M.

use crate::models::*;
use highs::{ColProblem, HighsModelStatus, Model, Row, Sense};
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

// ─── Planned variable descriptors ───────────────────────────
// Built in Phase 1, before any columns are added to ColProblem.

struct PlannedProd {
    pid: String, lid: String, rid: String, period: u32,
    obj_cost: f64, base_cost: f64,
    setup_cost: f64, min_lot: f64, big_m: f64, needs_binary: bool,
}

struct PlannedTrans {
    pid: String, from: String, to: String, period: u32,
    obj_cost: f64, base_unit_cost: f64,
    fixed_cost: f64, min_lot: f64, max_lot: f64, big_m: f64,
    needs_binary: bool, mode: TransportMode,
}

// ─── Pre-computed lookup tables ─────────────────────────────

struct Lookups {
    lead_times: HashMap<(String, String), u32>,
    resource_cost: HashMap<String, f64>,
    transport_costs: HashMap<(String, String), (f64, f64)>,
}

fn build_lookups(model: &SupplyChainModel) -> Lookups {
    let mut lead_times = HashMap::new();
    let mut transport_costs = HashMap::new();
    for tl in &model.transport_lanes {
        lead_times.insert(
            (tl.from_location_id.clone(), tl.to_location_id.clone()),
            tl.lead_time_periods,
        );
        transport_costs.insert(
            (tl.from_location_id.clone(), tl.to_location_id.clone()),
            (tl.cost_per_unit, tl.tariff_rate),
        );
    }
    let mut resource_cost = HashMap::new();
    for r in &model.resources {
        resource_cost.insert(r.id.clone(), r.cost_per_unit);
    }
    Lookups { lead_times, resource_cost, transport_costs }
}

// ─── Build and Solve (MILP) ─────────────────────────────────

fn build_and_solve(
    model: &SupplyChainModel,
    config: &SolverConfig,
) -> Result<SolverResult, String> {
    let np = model.num_periods;
    let mut pb = ColProblem::new();
    let lookups = build_lookups(model);

    // ═══════════════════════════════════════════════════════════
    // PHASE 1: PLAN VARIABLES (determine what will exist)
    // ═══════════════════════════════════════════════════════════

    // ─── Production variables ───
    let mut planned_prods: Vec<PlannedProd> = Vec::new();
    for sr in &model.sourcing_rules {
        if sr.sourcing_type != SourcingType::Production || !sr.active { continue; }
        let res_id = match &sr.resource_id { Some(r) => r, None => continue };
        let base_cost = lookups.resource_cost.get(res_id)
            .copied().unwrap_or(model.cost_profile.production_cost);
        let resource = model.resources.iter().find(|r| r.id == *res_id);
        let setup_cost = resource.map(|r| r.setup_cost).unwrap_or(0.0);
        let min_lot = sr.min_lot_size;

        for t in 0..np {
            let zone = fence_zone(t, config);
            let obj_cost = match zone {
                FenceZone::Frozen => base_cost,
                FenceZone::Firm => base_cost + config.firm_change_penalty,
                FenceZone::Free => base_cost,
            };
            let big_m = resource
                .map(|r| model.effective_capacity(r, t))
                .unwrap_or(1e9);
            let needs_binary = setup_cost > 0.0 || min_lot > 0.0;
            planned_prods.push(PlannedProd {
                pid: sr.product_id.clone(), lid: sr.from_location_id.clone(),
                rid: res_id.clone(), period: t,
                obj_cost, base_cost, setup_cost, min_lot, big_m, needs_binary,
            });
        }
    }

    // ─── Transport variables ───
    let mut planned_trans: Vec<PlannedTrans> = Vec::new();
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
                .copied().unwrap_or((model.cost_profile.transport_cost, 0.0));
            let base_unit_cost = unit_cost * (1.0 + tariff);

            for t in 0..np {
                let zone = fence_zone(t, config);
                let obj_cost = match zone {
                    FenceZone::Frozen => base_unit_cost,
                    FenceZone::Firm => base_unit_cost + config.firm_change_penalty,
                    FenceZone::Free => base_unit_cost,
                };
                let max_lot = if tl.max_lot_size >= f64::MAX * 0.5 { 1e9 } else { tl.max_lot_size };
                let big_m = max_lot;
                let needs_binary = tl.fixed_cost_per_shipment > 0.0 || tl.min_lot_size > 0.0;
                planned_trans.push(PlannedTrans {
                    pid: prod_id.clone(), from: tl.from_location_id.clone(),
                    to: tl.to_location_id.clone(), period: t,
                    obj_cost, base_unit_cost,
                    fixed_cost: tl.fixed_cost_per_shipment,
                    min_lot: tl.min_lot_size, max_lot, big_m, needs_binary,
                    mode: tl.mode.clone(),
                });
            }
        }
    }

    if planned_prods.is_empty() && planned_trans.is_empty() {
        return Err("No decision variables created. Check sourcing rules.".into());
    }

    // ═══════════════════════════════════════════════════════════
    // PHASE 2: CREATE ALL CONSTRAINT ROWS
    // ═══════════════════════════════════════════════════════════

    // 1. INVENTORY BALANCE rows (equality per PL × period)
    let mut inv_rows: HashMap<(String, String, u32), Row> = HashMap::new();
    for pl in &model.product_locations {
        if !pl.active { continue; }
        for t in 0..np {
            let initial = if t == 0 { pl.initial_inventory } else { 0.0 };
            let demand_at_t: f64 = model.demands.iter()
                .filter(|d| d.product_id == pl.product_id && d.location_id == pl.location_id && d.period == t)
                .map(|d| d.quantity).sum();
            let receipts_at_t: f64 = model.planned_receipts.iter()
                .filter(|r| r.product_id == pl.product_id && r.location_id == pl.location_id && r.period == t)
                .map(|r| r.quantity).sum();
            let rhs = initial - demand_at_t + receipts_at_t;
            let row = pb.add_row(rhs..=rhs);
            inv_rows.insert((pl.product_id.clone(), pl.location_id.clone(), t), row);
        }
    }

    // 2. LEGACY DEMAND rows (demands at locations without ProductLocation)
    let mut legacy_rows: HashMap<String, Row> = HashMap::new();
    for demand in &model.demands {
        let has_pl = model.product_locations.iter()
            .any(|pl| pl.product_id == demand.product_id && pl.location_id == demand.location_id && pl.active);
        if !has_pl {
            let row = pb.add_row(demand.quantity..);
            legacy_rows.insert(demand.id.clone(), row);
        }
    }

    // 3. CAPACITY rows (per resource × period)
    let mut cap_rows: HashMap<(String, u32), Row> = HashMap::new();
    for resource in &model.resources {
        if !resource.active { continue; }
        for t in 0..np {
            let eff = model.effective_capacity(resource, t);
            if eff <= 0.0 { continue; }
            let row = pb.add_row(..=eff);
            cap_rows.insert((resource.id.clone(), t), row);
        }
    }

    // 4. SAFETY STOCK rows: Inv + ss_viol >= safety_stock
    let mut ss_rows: HashMap<(String, String, u32), Row> = HashMap::new();
    for pl in &model.product_locations {
        if !pl.active || pl.safety_stock <= 0.0 { continue; }
        for t in 0..np {
            let row = pb.add_row(pl.safety_stock..);
            ss_rows.insert((pl.product_id.clone(), pl.location_id.clone(), t), row);
        }
    }

    // 5. MAX STOCK rows: Inv - ms_viol <= max_stock
    let mut ms_rows: HashMap<(String, String, u32), Row> = HashMap::new();
    for pl in &model.product_locations {
        if !pl.active || pl.max_stock >= f64::MAX * 0.5 { continue; }
        for t in 0..np {
            let row = pb.add_row(..=pl.max_stock);
            ms_rows.insert((pl.product_id.clone(), pl.location_id.clone(), t), row);
        }
    }

    // 6. FROZEN ZONE rows (production = 0)
    let mut frozen_prod: HashMap<usize, Row> = HashMap::new();
    for (i, pv) in planned_prods.iter().enumerate() {
        if fence_zone(pv.period, config) == FenceZone::Frozen {
            frozen_prod.insert(i, pb.add_row(0.0..=0.0));
        }
    }
    let mut frozen_trans: HashMap<usize, Row> = HashMap::new();
    for (i, tv) in planned_trans.iter().enumerate() {
        if fence_zone(tv.period, config) == FenceZone::Frozen {
            frozen_trans.insert(i, pb.add_row(0.0..=0.0));
        }
    }

    // 7. SETUP LINKING rows (big-M): prod_qty <= M * y_setup
    //    i.e. prod_qty - M*y_setup <= 0
    let mut setup_bigm: HashMap<usize, Row> = HashMap::new();
    let mut setup_minlot: HashMap<usize, (Row, f64)> = HashMap::new();
    for (i, pv) in planned_prods.iter().enumerate() {
        if !pv.needs_binary { continue; }
        setup_bigm.insert(i, pb.add_row(..=0.0));
        if pv.min_lot > 0.0 {
            setup_minlot.insert(i, (pb.add_row(0.0..), pv.min_lot));
        }
    }

    // 8. TRANSPORT ACTIVATION rows (big-M): trans_qty <= M * y_trans
    let mut trans_bigm: HashMap<usize, Row> = HashMap::new();
    let mut trans_minlot: HashMap<usize, (Row, f64)> = HashMap::new();
    for (i, tv) in planned_trans.iter().enumerate() {
        if !tv.needs_binary { continue; }
        trans_bigm.insert(i, pb.add_row(..=0.0));
        if tv.min_lot > 0.0 {
            trans_minlot.insert(i, (pb.add_row(0.0..), tv.min_lot));
        }
    }

    // ═══════════════════════════════════════════════════════════
    // PHASE 3: ADD DECISION VARIABLES (COLUMNS)
    // ═══════════════════════════════════════════════════════════

    let mut col_count: usize = 0;

    // ─── 3a. Production variables (continuous) ───
    let mut prod_indices: Vec<usize> = Vec::new();
    for (i, pv) in planned_prods.iter().enumerate() {
        let mut rc: Vec<(Row, f64)> = Vec::new();
        let yield_rate = model.get_yield_rate(&pv.pid, &pv.lid);

        // Inventory balance: production ADDS to inventory → coeff = -yield_rate
        if let Some(&row) = inv_rows.get(&(pv.pid.clone(), pv.lid.clone(), pv.period)) {
            rc.push((row, -yield_rate));
        }
        // BOM consumption: producing output consumes inputs
        for bom in &model.bom_entries {
            if bom.output_product_id == pv.pid
                && (bom.location_id.is_none() || bom.location_id.as_deref() == Some(&pv.lid))
            {
                if let Some(&row) = inv_rows.get(&(bom.input_product_id.clone(), pv.lid.clone(), pv.period)) {
                    rc.push((row, bom.quantity_per));
                }
            }
        }
        // Capacity constraint
        if let Some(&row) = cap_rows.get(&(pv.rid.clone(), pv.period)) {
            let rate = model.get_consumption_rate(&pv.pid, &pv.rid);
            rc.push((row, rate));
        }
        // Legacy demand satisfaction
        for demand in &model.demands {
            if demand.product_id == pv.pid && demand.location_id == pv.lid && demand.period == pv.period {
                if let Some(&row) = legacy_rows.get(&demand.id) {
                    rc.push((row, yield_rate));
                }
            }
        }
        // Frozen zone
        if let Some(&row) = frozen_prod.get(&i) { rc.push((row, 1.0)); }
        // Setup big-M: coeff +1.0 in (prod - M*y <= 0) row
        if let Some(&row) = setup_bigm.get(&i) { rc.push((row, 1.0)); }
        // Min lot: coeff +1.0 in (prod - minlot*y >= 0) row
        if let Some(&(row, _)) = setup_minlot.get(&i) { rc.push((row, 1.0)); }

        pb.add_column(pv.obj_cost, 0.0.., &rc);
        prod_indices.push(col_count);
        col_count += 1;
    }

    // ─── 3b. Production setup binary variables ───
    let mut setup_indices: Vec<(usize, usize)> = Vec::new(); // (prod_var_idx, col_idx)
    for (i, pv) in planned_prods.iter().enumerate() {
        if !pv.needs_binary { continue; }
        let mut rc: Vec<(Row, f64)> = Vec::new();
        // Big-M row: coeff = -M
        if let Some(&row) = setup_bigm.get(&i) { rc.push((row, -pv.big_m)); }
        // Min lot row: coeff = -min_lot
        if let Some(&(row, ml)) = setup_minlot.get(&i) { rc.push((row, -ml)); }

        pb.add_integer_column(pv.setup_cost, 0.0..=1.0, &rc);
        setup_indices.push((i, col_count));
        col_count += 1;
    }

    // ─── 3c. Transport variables (continuous) ───
    let mut trans_indices: Vec<usize> = Vec::new();
    for (i, tv) in planned_trans.iter().enumerate() {
        let mut rc: Vec<(Row, f64)> = Vec::new();
        let lead = lookups.lead_times
            .get(&(tv.from.clone(), tv.to.clone())).copied().unwrap_or(0);

        // Outbound: removes from source inventory
        if let Some(&row) = inv_rows.get(&(tv.pid.clone(), tv.from.clone(), tv.period)) {
            rc.push((row, 1.0));
        }
        // Inbound: adds to destination after lead time
        let arrival = tv.period + lead;
        if arrival < np {
            if let Some(&row) = inv_rows.get(&(tv.pid.clone(), tv.to.clone(), arrival)) {
                rc.push((row, -1.0));
            }
        }
        // Legacy demand satisfaction
        for demand in &model.demands {
            if demand.product_id == tv.pid && demand.location_id == tv.to {
                if let Some(&row) = legacy_rows.get(&demand.id) {
                    if tv.period + lead == demand.period {
                        rc.push((row, 1.0));
                    }
                }
            }
        }
        // Frozen zone
        if let Some(&row) = frozen_trans.get(&i) { rc.push((row, 1.0)); }
        // Transport activation big-M
        if let Some(&row) = trans_bigm.get(&i) { rc.push((row, 1.0)); }
        // Transport min lot
        if let Some(&(row, _)) = trans_minlot.get(&i) { rc.push((row, 1.0)); }

        let upper = if tv.max_lot >= 1e9 { f64::INFINITY } else { tv.max_lot };
        pb.add_column(tv.obj_cost, 0.0..=upper, &rc);
        trans_indices.push(col_count);
        col_count += 1;
    }

    // ─── 3d. Transport activation binary variables ───
    let mut trans_act_indices: Vec<(usize, usize)> = Vec::new();
    for (i, tv) in planned_trans.iter().enumerate() {
        if !tv.needs_binary { continue; }
        let mut rc: Vec<(Row, f64)> = Vec::new();
        if let Some(&row) = trans_bigm.get(&i) { rc.push((row, -tv.big_m)); }
        if let Some(&(row, ml)) = trans_minlot.get(&i) { rc.push((row, -ml)); }

        pb.add_integer_column(tv.fixed_cost, 0.0..=1.0, &rc);
        trans_act_indices.push((i, col_count));
        col_count += 1;
    }

    // ─── 3e. Inventory variables (continuous) ───
    let mut inv_indices: Vec<(String, String, u32, usize)> = Vec::new();
    for pl in &model.product_locations {
        if !pl.active { continue; }
        for t in 0..np {
            let mut rc: Vec<(Row, f64)> = Vec::new();
            // Inv[t] in balance row for period t
            if let Some(&row) = inv_rows.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                rc.push((row, 1.0));
            }
            // -Inv[t] in balance row for period t+1 (carry-forward)
            if t + 1 < np {
                if let Some(&row) = inv_rows.get(&(pl.product_id.clone(), pl.location_id.clone(), t + 1)) {
                    rc.push((row, -1.0));
                }
            }
            // Safety stock row
            if let Some(&row) = ss_rows.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                rc.push((row, 1.0));
            }
            // Max stock row
            if let Some(&row) = ms_rows.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                rc.push((row, 1.0));
            }
            pb.add_column(pl.holding_cost_per_unit, 0.0.., &rc);
            inv_indices.push((pl.product_id.clone(), pl.location_id.clone(), t, col_count));
            col_count += 1;
        }
    }

    // ─── 3f. Unmet demand variables (continuous) ───
    let mut unmet_indices: Vec<(String, usize)> = Vec::new();
    for demand in &model.demands {
        let mut rc: Vec<(Row, f64)> = Vec::new();
        // In inventory balance: unmet REDUCES the demand obligation
        if let Some(&row) = inv_rows.get(&(demand.product_id.clone(), demand.location_id.clone(), demand.period)) {
            rc.push((row, -1.0));
        }
        // In legacy demand row
        if let Some(&row) = legacy_rows.get(&demand.id) {
            rc.push((row, 1.0));
        }
        pb.add_column(demand.non_delivery_cost, 0.0.., &rc);
        unmet_indices.push((demand.id.clone(), col_count));
        col_count += 1;
    }

    // ─── 3g. Safety stock violation variables ───
    let mut ss_viol_indices: Vec<usize> = Vec::new();
    for pl in &model.product_locations {
        if !pl.active || pl.safety_stock <= 0.0 { continue; }
        for t in 0..np {
            let mut rc: Vec<(Row, f64)> = Vec::new();
            if let Some(&row) = ss_rows.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                rc.push((row, 1.0));
            }
            pb.add_column(pl.safety_stock_violation_cost, 0.0.., &rc);
            ss_viol_indices.push(col_count);
            col_count += 1;
        }
    }

    // ─── 3h. Max stock violation variables ───
    let mut ms_viol_indices: Vec<usize> = Vec::new();
    for pl in &model.product_locations {
        if !pl.active || pl.max_stock >= f64::MAX * 0.5 { continue; }
        for t in 0..np {
            let mut rc: Vec<(Row, f64)> = Vec::new();
            if let Some(&row) = ms_rows.get(&(pl.product_id.clone(), pl.location_id.clone(), t)) {
                rc.push((row, -1.0));
            }
            pb.add_column(pl.max_stock_violation_cost, 0.0.., &rc);
            ms_viol_indices.push(col_count);
            col_count += 1;
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
    let vals = solution.columns();

    // Production plan
    let mut production_plan = Vec::new();
    let mut total_production_cost = 0.0;
    for (i, pv) in planned_prods.iter().enumerate() {
        let qty = vals[prod_indices[i]];
        if qty > 0.001 {
            let cost = qty * pv.base_cost;
            total_production_cost += cost;
            production_plan.push(PlanEntry {
                product_id: pv.pid.clone(), location_id: pv.lid.clone(),
                resource_id: pv.rid.clone(), period: pv.period, quantity: qty, cost,
            });
        }
    }
    // Add setup costs
    for &(pi, ci) in &setup_indices {
        let y = vals[ci];
        if y > 0.5 {
            total_production_cost += planned_prods[pi].setup_cost;
        }
    }

    // Transport plan
    let mut transport_plan = Vec::new();
    let mut total_transport_cost = 0.0;
    for (i, tv) in planned_trans.iter().enumerate() {
        let qty = vals[trans_indices[i]];
        if qty > 0.001 {
            let cost = qty * tv.base_unit_cost;
            total_transport_cost += cost;
            transport_plan.push(TransportPlanEntry {
                product_id: tv.pid.clone(), from_location_id: tv.from.clone(),
                to_location_id: tv.to.clone(), period: tv.period,
                quantity: qty, cost, mode: tv.mode.clone(),
            });
        }
    }
    // Add fixed shipment costs
    for &(ti, ci) in &trans_act_indices {
        let y = vals[ci];
        if y > 0.5 {
            total_transport_cost += planned_trans[ti].fixed_cost;
        }
    }

    // Inventory plan
    let mut inventory_plan = Vec::new();
    let mut total_holding_cost = 0.0;
    let mut all_inv_values: Vec<f64> = Vec::new();
    for &(ref pid, ref lid, t, ci) in &inv_indices {
        let qty = vals[ci];
        let pl = model.get_product_location(pid, lid);
        let holding = pl.map(|p| p.holding_cost_per_unit).unwrap_or(0.5);
        let hcost = qty * holding;
        total_holding_cost += hcost;
        all_inv_values.push(qty);
        let ss = pl.map(|p| p.safety_stock).unwrap_or(0.0);
        inventory_plan.push(InventoryEntry {
            product_id: pid.clone(), location_id: lid.clone(), period: t,
            quantity: qty, holding_cost: hcost, safety_stock_delta: qty - ss,
        });
    }

    // Unmet demand
    let mut unmet_demand = Vec::new();
    let mut total_penalty_cost = 0.0;
    for &(ref did, ci) in &unmet_indices {
        let qty = vals[ci];
        if qty > 0.001 {
            if let Some(demand) = model.demands.iter().find(|d| d.id == *did) {
                let penalty = qty * demand.non_delivery_cost;
                total_penalty_cost += penalty;
                unmet_demand.push(UnmetDemandEntry {
                    demand_id: did.clone(), product_id: demand.product_id.clone(),
                    location_id: demand.location_id.clone(), period: demand.period,
                    unmet_quantity: qty, penalty_cost: penalty,
                    reason: "Insufficient capacity or too costly to deliver".into(),
                });
            }
        }
    }

    // Safety stock + max stock violation costs
    for &ci in &ss_viol_indices {
        let qty = vals[ci];
        if qty > 0.001 { total_penalty_cost += qty; }
    }
    for &ci in &ms_viol_indices {
        let qty = vals[ci];
        if qty > 0.001 { total_penalty_cost += qty; }
    }

    // Capacity utilization
    let mut capacity_utilization = Vec::new();
    for resource in &model.resources {
        if !resource.active { continue; }
        for t in 0..np {
            let eff = model.effective_capacity(resource, t);
            let mut used = 0.0;
            for (i, pv) in planned_prods.iter().enumerate() {
                if pv.lid == resource.location_id && pv.rid == resource.id && pv.period == t {
                    let rate = model.get_consumption_rate(&pv.pid, &resource.id);
                    used += vals[prod_indices[i]] * rate;
                }
            }
            let util_pct = if eff > 0.0 { (used / eff * 100.0).min(100.0) } else { 0.0 };
            capacity_utilization.push(CapacityUtilEntry {
                resource_id: resource.id.clone(), location_id: resource.location_id.clone(),
                period: t, used, available: eff, utilization_pct: util_pct,
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
