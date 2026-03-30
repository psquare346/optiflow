// OptiFlow — Rule-Based Decision Explainer
// Answers natural-language questions about optimizer results by
// cross-referencing solver output with master/transaction data.

use serde::{Deserialize, Serialize};
use crate::models::*;

// ─── Response Types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainResponse {
    pub intent: String,
    pub answer: String,
    pub data_points: Vec<DataPoint>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub label: String,
    pub value: String,
    pub context: Option<String>,
}

// ─── Intent Classification ──────────────────────────────────

#[derive(Debug)]
enum Intent {
    Deployment { location: Option<String>, product: Option<String> },
    UnmetDemand { location: Option<String>, product: Option<String> },
    Capacity { location: Option<String>, resource: Option<String> },
    Inventory { location: Option<String>, product: Option<String> },
    Cost,
    Production { location: Option<String>, product: Option<String> },
    General,
}

fn classify_intent(question: &str, model: &SupplyChainModel) -> Intent {
    let q = question.to_lowercase();

    // Extract entities from question
    let loc = extract_location(&q, model);
    let prod = extract_product(&q, model);
    let res = extract_resource(&q, model);

    if q.contains("unmet") || q.contains("unfulfill") || q.contains("not met")
        || q.contains("shortfall") || q.contains("couldn't deliver")
        || q.contains("short") || q.contains("not deliver")
        || q.contains("missed") || q.contains("backlog")
    {
        Intent::UnmetDemand { location: loc, product: prod }
    } else if q.contains("deploy") || q.contains("transport") || q.contains("ship")
        || q.contains("shipment") || q.contains("sending") || q.contains("moving")
        || q.contains("logistics") || q.contains("freight")
    {
        Intent::Deployment { location: loc, product: prod }
    } else if q.contains("capacity") || q.contains("utiliz") || q.contains("bottleneck")
        || q.contains("resource") || q.contains("overload") || q.contains("constraint")
        || q.contains("maxed out") || q.contains("full")
    {
        Intent::Capacity { location: loc, resource: res }
    } else if q.contains("inventory") || q.contains("stock") || q.contains("building up")
        || q.contains("holding") || q.contains("warehouse") || q.contains("buffer")
        || q.contains("safety") || q.contains("storage")
    {
        Intent::Inventory { location: loc, product: prod }
    } else if q.contains("cost") || q.contains("expensive") || q.contains("spending")
        || q.contains("budget") || q.contains("price") || q.contains("profit")
        || q.contains("margin") || q.contains("penalty")
    {
        Intent::Cost
    } else if q.contains("produc") || q.contains("manufactur") || q.contains("making")
        || q.contains("fabricat") || q.contains("output") || q.contains("factory")
        || q.contains("plant")
    {
        Intent::Production { location: loc, product: prod }
    } else {
        Intent::General
    }
}

fn extract_location(q: &str, model: &SupplyChainModel) -> Option<String> {
    // Match location IDs first (exact), then names (fuzzy)
    for loc in &model.locations {
        if q.contains(&loc.id.to_lowercase()) {
            return Some(loc.id.clone());
        }
    }
    for loc in &model.locations {
        let name_lower = loc.name.to_lowercase();
        // Check individual words from name (skip very short words)
        for word in name_lower.split_whitespace() {
            if word.len() >= 4 && q.contains(word) {
                return Some(loc.id.clone());
            }
        }
    }
    // Check country/region
    for loc in &model.locations {
        if q.contains(&loc.country.to_lowercase()) {
            return Some(loc.id.clone());
        }
    }
    None
}

fn extract_product(q: &str, model: &SupplyChainModel) -> Option<String> {
    for prod in &model.products {
        if q.contains(&prod.id.to_lowercase()) {
            return Some(prod.id.clone());
        }
    }
    for prod in &model.products {
        let name_lower = prod.name.to_lowercase();
        for word in name_lower.split_whitespace() {
            if word.len() >= 3 && q.contains(word) {
                return Some(prod.id.clone());
            }
        }
    }
    None
}

fn extract_resource(q: &str, model: &SupplyChainModel) -> Option<String> {
    for res in &model.resources {
        if q.contains(&res.id.to_lowercase()) {
            return Some(res.id.clone());
        }
    }
    for res in &model.resources {
        let name_lower = res.name.to_lowercase();
        for word in name_lower.split_whitespace() {
            if word.len() >= 4 && q.contains(word) {
                return Some(res.id.clone());
            }
        }
    }
    None
}

// ─── Main Entry Point ───────────────────────────────────────

pub fn explain(
    question: &str,
    model: &SupplyChainModel,
    result: &SolverResult,
) -> ExplainResponse {
    let intent = classify_intent(question, model);
    match intent {
        Intent::Deployment { location, product } => explain_deployment(model, result, location, product),
        Intent::UnmetDemand { location, product } => explain_unmet(model, result, location, product),
        Intent::Capacity { location, resource } => explain_capacity(model, result, location, resource),
        Intent::Inventory { location, product } => explain_inventory(model, result, location, product),
        Intent::Cost => explain_cost(model, result),
        Intent::Production { location, product } => explain_production(model, result, location, product),
        Intent::General => explain_general(model, result),
    }
}

// ─── Formatting Helpers ─────────────────────────────────────

fn fmt_qty(n: f64) -> String {
    if n == 0.0 { return "0".into(); }
    if n >= 1000.0 {
        format!("{}", (n as i64).to_string()
            .as_bytes().rchunks(3).rev()
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>().join(","))
    } else {
        format!("{:.1}", n)
    }
}

fn fmt_cost(n: f64) -> String {
    format!("${}", fmt_qty(n))
}

fn loc_name(model: &SupplyChainModel, id: &str) -> String {
    model.locations.iter().find(|l| l.id == id)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| id.to_string())
}

fn prod_name(model: &SupplyChainModel, id: &str) -> String {
    model.products.iter().find(|p| p.id == id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| id.to_string())
}

fn res_name(model: &SupplyChainModel, id: &str) -> String {
    model.resources.iter().find(|r| r.id == id)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| id.to_string())
}

// ─── Deployment / Transport Explanation ─────────────────────

fn explain_deployment(
    model: &SupplyChainModel,
    result: &SolverResult,
    location: Option<String>,
    product: Option<String>,
) -> ExplainResponse {
    let mut data_points = Vec::new();
    let mut lines = Vec::new();

    // Filter transport plan
    let transports: Vec<_> = result.transport_plan.iter().filter(|t| {
        let loc_match = location.as_ref().map_or(true, |l| &t.to_location_id == l || &t.from_location_id == l);
        let prod_match = product.as_ref().map_or(true, |p| &t.product_id == p);
        loc_match && prod_match
    }).collect();

    if transports.is_empty() {
        return ExplainResponse {
            intent: "deployment".into(),
            answer: "No transport/deployment found matching your query. The optimizer did not plan any shipments for the specified criteria.".into(),
            data_points: vec![],
            suggestions: vec!["Try asking about a specific location or product".into()],
        };
    }

    // Aggregate by destination
    let mut by_dest: std::collections::HashMap<String, (f64, f64)> = std::collections::HashMap::new();
    for t in &transports {
        let entry = by_dest.entry(t.to_location_id.clone()).or_insert((0.0, 0.0));
        entry.0 += t.quantity;
        entry.1 += t.cost;
    }

    // Compare against demand at each destination
    for (dest, (total_qty, total_cost)) in &by_dest {
        let dest_name = loc_name(model, dest);

        // Demand at this location
        let demand_at_dest: f64 = model.demands.iter()
            .filter(|d| &d.location_id == dest && product.as_ref().map_or(true, |p| &d.product_id == p))
            .map(|d| d.quantity)
            .sum();

        // Safety stock needs
        let ss: f64 = model.product_locations.iter()
            .filter(|pl| &pl.location_id == dest && product.as_ref().map_or(true, |p| &pl.product_id == p))
            .map(|pl| pl.safety_stock)
            .sum();

        // Initial inventory
        let init_inv: f64 = model.product_locations.iter()
            .filter(|pl| &pl.location_id == dest && product.as_ref().map_or(true, |p| &pl.product_id == p))
            .map(|pl| pl.initial_inventory)
            .sum();

        // Does this location serve downstream?
        let downstream_lanes: Vec<_> = model.transport_lanes.iter()
            .filter(|tl| tl.from_location_id == *dest && tl.active)
            .collect();

        let mut explanation = format!("📦 **{}** receives {} units (cost: {})", dest_name, fmt_qty(*total_qty), fmt_cost(*total_cost));

        data_points.push(DataPoint {
            label: format!("Total deployed to {}", dest_name),
            value: fmt_qty(*total_qty),
            context: Some(fmt_cost(*total_cost)),
        });
        data_points.push(DataPoint {
            label: format!("Total demand at {}", dest_name),
            value: fmt_qty(demand_at_dest),
            context: None,
        });

        if demand_at_dest > 0.0 {
            explanation += &format!("\n• Direct demand here: {} units across all periods", fmt_qty(demand_at_dest));
        }
        if init_inv > 0.0 {
            explanation += &format!("\n• Opening inventory: {} units", fmt_qty(init_inv));
            data_points.push(DataPoint {
                label: format!("Initial inventory at {}", dest_name),
                value: fmt_qty(init_inv),
                context: None,
            });
        }
        if ss > 0.0 {
            explanation += &format!("\n• Safety stock requirement: {} units — optimizer builds extra stock to maintain this buffer", fmt_qty(ss));
            data_points.push(DataPoint {
                label: "Safety stock requirement".into(),
                value: fmt_qty(ss),
                context: Some("Optimizer pre-builds to maintain buffer".into()),
            });
        }
        if !downstream_lanes.is_empty() {
            let dests: Vec<String> = downstream_lanes.iter()
                .map(|tl| loc_name(model, &tl.to_location_id))
                .collect();
            explanation += &format!("\n• This location also serves downstream: {} — some deployment covers onward shipments", dests.join(", "));
        }
        if *total_qty > demand_at_dest && demand_at_dest > 0.0 {
            explanation += &format!("\n\n⚡ The deployment ({}) exceeds direct demand ({}) because the optimizer accounts for safety stock buffers{} across multiple periods.",
                fmt_qty(*total_qty), fmt_qty(demand_at_dest),
                if !downstream_lanes.is_empty() { " and downstream distribution" } else { "" }
            );
        }

        lines.push(explanation);
    }

    ExplainResponse {
        intent: "deployment".into(),
        answer: lines.join("\n\n"),
        data_points,
        suggestions: vec![
            "Why is there unmet demand?".into(),
            "What are the bottleneck resources?".into(),
        ],
    }
}

// ─── Unmet Demand Explanation ───────────────────────────────

fn explain_unmet(
    model: &SupplyChainModel,
    result: &SolverResult,
    location: Option<String>,
    product: Option<String>,
) -> ExplainResponse {
    let unmet: Vec<_> = result.unmet_demand.iter().filter(|u| {
        let loc_match = location.as_ref().map_or(true, |l| &u.location_id == l);
        let prod_match = product.as_ref().map_or(true, |p| &u.product_id == p);
        loc_match && prod_match
    }).collect();

    if unmet.is_empty() {
        return ExplainResponse {
            intent: "unmet_demand".into(),
            answer: "✅ **All demand is fulfilled!** The optimizer was able to meet 100% of demand for the specified criteria.".into(),
            data_points: vec![DataPoint {
                label: "Demand Fulfillment".into(),
                value: format!("{:.1}%", result.kpis.demand_fulfillment_pct),
                context: None,
            }],
            suggestions: vec![
                "What is driving the cost?".into(),
                "Which resources are bottlenecks?".into(),
            ],
        };
    }

    let mut data_points = Vec::new();
    let mut lines = Vec::new();
    let total_unmet: f64 = unmet.iter().map(|u| u.unmet_quantity).sum();
    let total_penalty: f64 = unmet.iter().map(|u| u.penalty_cost).sum();

    lines.push(format!("⚠️ **{} units of demand are unmet** (penalty: {})", fmt_qty(total_unmet), fmt_cost(total_penalty)));

    data_points.push(DataPoint {
        label: "Total unmet quantity".into(),
        value: fmt_qty(total_unmet),
        context: Some(format!("Penalty: {}", fmt_cost(total_penalty))),
    });

    // Group by product-location
    let mut by_pl: std::collections::HashMap<(String, String), Vec<&UnmetDemandEntry>> = std::collections::HashMap::new();
    for u in &unmet {
        by_pl.entry((u.product_id.clone(), u.location_id.clone())).or_default().push(u);
    }

    for ((pid, lid), entries) in &by_pl {
        let p_name = prod_name(model, pid);
        let l_name = loc_name(model, lid);
        let qty: f64 = entries.iter().map(|e| e.unmet_quantity).sum();
        let periods: Vec<String> = entries.iter().map(|e| format!("P{}", e.period + 1)).collect();

        lines.push(format!("\n**{} at {}**: {} unmet in {}", p_name, l_name, fmt_qty(qty), periods.join(", ")));

        // Check upstream capacity
        let upstream_resources: Vec<_> = model.resources.iter().filter(|r| {
            model.sourcing_rules.iter().any(|sr| {
                sr.product_id == *pid && sr.from_location_id == r.location_id && sr.active
                    && sr.resource_id.as_deref() == Some(&r.id)
            })
        }).collect();

        for res in &upstream_resources {
            let utils: Vec<_> = result.capacity_utilization.iter()
                .filter(|c| c.resource_id == res.id)
                .collect();
            let avg_util: f64 = if utils.is_empty() { 0.0 } else {
                utils.iter().map(|c| c.utilization_pct).sum::<f64>() / utils.len() as f64
            };
            let max_util = utils.iter().map(|c| c.utilization_pct).fold(0.0_f64, f64::max);

            if max_util > 85.0 {
                lines.push(format!("  → 🔴 {} at {} is at {:.0}% peak utilization — **capacity constrained**",
                    res_name(model, &res.id), loc_name(model, &res.location_id), max_util));
                data_points.push(DataPoint {
                    label: format!("{} peak utilization", res_name(model, &res.id)),
                    value: format!("{:.1}%", max_util),
                    context: Some(format!("Avg: {:.1}%", avg_util)),
                });
            }
        }

        // Check transport lane availability
        let inbound_lanes: Vec<_> = model.transport_lanes.iter()
            .filter(|tl| tl.to_location_id == *lid && tl.active)
            .collect();
        if inbound_lanes.is_empty() {
            lines.push(format!("  → 🚫 No active transport lanes deliver to {} — this location may be unreachable", l_name));
        }

        // Check non-delivery cost vs production cost
        for entry in entries {
            if let Some(demand) = model.demands.iter().find(|d| d.id == entry.demand_id) {
                // Find cheapest production + transport path
                let cheapest_prod = model.resources.iter()
                    .filter(|r| r.location_id == *lid || model.transport_lanes.iter().any(|tl| tl.to_location_id == *lid && tl.from_location_id == r.location_id))
                    .map(|r| r.cost_per_unit)
                    .fold(f64::MAX, f64::min);

                if cheapest_prod < f64::MAX && demand.non_delivery_cost < cheapest_prod {
                    lines.push(format!("  → 💡 Non-delivery penalty ({}/unit) is lower than cheapest production cost ({}/unit) — **it's cheaper to skip this demand**",
                        fmt_cost(demand.non_delivery_cost), fmt_cost(cheapest_prod)));
                    break; // Only show once per product-location
                }
            }
        }
    }

    ExplainResponse {
        intent: "unmet_demand".into(),
        answer: lines.join("\n"),
        data_points,
        suggestions: vec![
            "What are the bottleneck resources?".into(),
            "How can I reduce unmet demand?".into(),
        ],
    }
}

// ─── Capacity Explanation ───────────────────────────────────

fn explain_capacity(
    model: &SupplyChainModel,
    result: &SolverResult,
    location: Option<String>,
    resource: Option<String>,
) -> ExplainResponse {
    let caps: Vec<_> = result.capacity_utilization.iter().filter(|c| {
        let loc_match = location.as_ref().map_or(true, |l| &c.location_id == l);
        let res_match = resource.as_ref().map_or(true, |r| &c.resource_id == r);
        loc_match && res_match
    }).collect();

    if caps.is_empty() {
        return ExplainResponse {
            intent: "capacity".into(),
            answer: "No capacity data found for the specified criteria.".into(),
            data_points: vec![],
            suggestions: vec!["Try asking about a specific resource or location".into()],
        };
    }

    let mut data_points = Vec::new();
    let mut lines = Vec::new();

    // Group by resource
    let mut by_res: std::collections::HashMap<String, Vec<&CapacityUtilEntry>> = std::collections::HashMap::new();
    for c in &caps {
        by_res.entry(c.resource_id.clone()).or_default().push(c);
    }

    for (rid, entries) in &by_res {
        let r_name = res_name(model, rid);
        let l_name = loc_name(model, &entries[0].location_id);
        let avg_util: f64 = entries.iter().map(|e| e.utilization_pct).sum::<f64>() / entries.len() as f64;
        let max_util = entries.iter().map(|e| e.utilization_pct).fold(0.0_f64, f64::max);
        let total_used: f64 = entries.iter().map(|e| e.used).sum();
        let total_avail: f64 = entries.iter().map(|e| e.available).sum();

        let status = if max_util > 95.0 { "🔴 BOTTLENECK" }
            else if max_util > 80.0 { "🟡 HIGH" }
            else { "🟢 HEALTHY" };

        lines.push(format!("**{} at {}** — {} ({:.1}% avg, {:.1}% peak)", r_name, l_name, status, avg_util, max_util));

        data_points.push(DataPoint {
            label: format!("{} avg utilization", r_name),
            value: format!("{:.1}%", avg_util),
            context: Some(format!("Peak: {:.1}%", max_util)),
        });
        data_points.push(DataPoint {
            label: format!("{} total used / available", r_name),
            value: format!("{} / {}", fmt_qty(total_used), fmt_qty(total_avail)),
            context: None,
        });

        // Show which products consume this resource
        let consumers: Vec<_> = result.production_plan.iter()
            .filter(|p| p.resource_id == *rid)
            .collect();

        let mut by_product: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for c in &consumers {
            *by_product.entry(c.product_id.clone()).or_insert(0.0) += c.quantity;
        }

        if !by_product.is_empty() {
            lines.push("  Products consuming this resource:".into());
            let mut sorted: Vec<_> = by_product.iter().collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            for (pid, qty) in sorted {
                let pct = if total_used > 0.0 { qty / total_used * 100.0 } else { 0.0 };
                let rate = model.get_consumption_rate(pid, rid);
                lines.push(format!("  • {} — {} units ({:.0}% of capacity, {:.1}x consumption rate)",
                    prod_name(model, pid), fmt_qty(*qty), pct, rate));
            }
        }

        // High utilization periods
        let hot_periods: Vec<String> = entries.iter()
            .filter(|e| e.utilization_pct > 90.0)
            .map(|e| format!("P{} ({:.0}%)", e.period + 1, e.utilization_pct))
            .collect();
        if !hot_periods.is_empty() {
            lines.push(format!("  ⚡ Hot periods: {}", hot_periods.join(", ")));
        }
    }

    let bottleneck_count = by_res.values()
        .filter(|entries| entries.iter().any(|e| e.utilization_pct > 95.0))
        .count();

    if bottleneck_count > 0 {
        lines.push(format!("\n💡 **{} resource(s) are bottlenecked.** Consider adding capacity, extending shifts, or adding alternate sourcing.", bottleneck_count));
    }

    ExplainResponse {
        intent: "capacity".into(),
        answer: lines.join("\n"),
        data_points,
        suggestions: vec![
            "Why is there unmet demand?".into(),
            "What is the cost breakdown?".into(),
        ],
    }
}

// ─── Inventory Explanation ──────────────────────────────────

fn explain_inventory(
    model: &SupplyChainModel,
    result: &SolverResult,
    location: Option<String>,
    product: Option<String>,
) -> ExplainResponse {
    let inv: Vec<_> = result.inventory_plan.iter().filter(|i| {
        let loc_match = location.as_ref().map_or(true, |l| &i.location_id == l);
        let prod_match = product.as_ref().map_or(true, |p| &i.product_id == p);
        loc_match && prod_match
    }).collect();

    if inv.is_empty() {
        return ExplainResponse {
            intent: "inventory".into(),
            answer: "No inventory data found. Make sure Product-Location records are defined.".into(),
            data_points: vec![],
            suggestions: vec!["Set up Product ↔ Location records to enable inventory tracking".into()],
        };
    }

    let mut data_points = Vec::new();
    let mut lines = Vec::new();

    // Group by product-location
    let mut by_pl: std::collections::HashMap<(String, String), Vec<&InventoryEntry>> = std::collections::HashMap::new();
    for i in &inv {
        by_pl.entry((i.product_id.clone(), i.location_id.clone())).or_default().push(i);
    }

    for ((pid, lid), entries) in &by_pl {
        let p_name = prod_name(model, pid);
        let l_name = loc_name(model, lid);
        let avg_stock: f64 = entries.iter().map(|e| e.quantity).sum::<f64>() / entries.len() as f64;
        let peak_stock = entries.iter().map(|e| e.quantity).fold(0.0_f64, f64::max);
        let min_stock = entries.iter().map(|e| e.quantity).fold(f64::MAX, f64::min);
        let total_holding: f64 = entries.iter().map(|e| e.holding_cost).sum();

        // Get master data
        let pl = model.get_product_location(pid, lid);
        let ss = pl.map(|p| p.safety_stock).unwrap_or(0.0);
        let max_stock = pl.map(|p| p.max_stock).unwrap_or(f64::MAX);

        let ss_violations: Vec<_> = entries.iter().filter(|e| e.safety_stock_delta < 0.0).collect();

        lines.push(format!("**{} at {}**:", p_name, l_name));
        lines.push(format!("  • Average stock: {} | Peak: {} | Min: {}", fmt_qty(avg_stock), fmt_qty(peak_stock), fmt_qty(min_stock)));
        lines.push(format!("  • Total holding cost: {}", fmt_cost(total_holding)));

        data_points.push(DataPoint {
            label: format!("{} at {} — avg stock", p_name, l_name),
            value: fmt_qty(avg_stock),
            context: Some(format!("Peak: {} | Min: {}", fmt_qty(peak_stock), fmt_qty(min_stock))),
        });

        if ss > 0.0 {
            if ss_violations.is_empty() {
                lines.push(format!("  • ✅ Safety stock ({}) maintained in all periods", fmt_qty(ss)));
            } else {
                let worst = ss_violations.iter().map(|e| e.safety_stock_delta).fold(0.0_f64, f64::min);
                lines.push(format!("  • ⚠️ Safety stock ({}) violated in {} periods (worst: {} below target)",
                    fmt_qty(ss), ss_violations.len(), fmt_qty(worst.abs())));
            }
        }

        if max_stock < f64::MAX * 0.5 {
            let over_max: Vec<_> = entries.iter().filter(|e| e.quantity > max_stock).collect();
            if !over_max.is_empty() {
                lines.push(format!("  • 🔴 Max stock ({}) exceeded in {} periods", fmt_qty(max_stock), over_max.len()));
            }
        }

        // Why is inventory high?
        if avg_stock > ss * 2.0 && ss > 0.0 {
            // Check if this location serves downstream
            let downstream: Vec<_> = model.transport_lanes.iter()
                .filter(|tl| tl.from_location_id == *lid && tl.active)
                .collect();
            if !downstream.is_empty() {
                let dests: Vec<String> = downstream.iter().map(|tl| loc_name(model, &tl.to_location_id)).collect();
                lines.push(format!("  → 💡 High inventory likely because this location serves as a hub for: {}", dests.join(", ")));
            }

            // Check planned receipts
            let receipts: f64 = model.planned_receipts.iter()
                .filter(|r| r.product_id == *pid && r.location_id == *lid)
                .map(|r| r.quantity)
                .sum();
            if receipts > 0.0 {
                lines.push(format!("  → 💡 {} units arriving via planned receipts (POs/in-transit)", fmt_qty(receipts)));
            }
        }
    }

    ExplainResponse {
        intent: "inventory".into(),
        answer: lines.join("\n"),
        data_points,
        suggestions: vec![
            "What is the cost breakdown?".into(),
            "Why is there transport to this location?".into(),
        ],
    }
}

// ─── Cost Explanation ───────────────────────────────────────

fn explain_cost(
    model: &SupplyChainModel,
    result: &SolverResult,
) -> ExplainResponse {
    let k = &result.kpis;
    let mut data_points = Vec::new();
    let mut lines = Vec::new();

    lines.push(format!("💰 **Total cost: {}** — here's the breakdown:", fmt_cost(k.total_cost)));

    let cost_items = vec![
        ("Production (manufacturing)", k.production_cost),
        ("Transport (logistics)", k.transport_cost),
        ("Holding (inventory storage)", k.holding_cost),
        ("Penalties (unmet demand + violations)", k.penalty_cost),
    ];

    for (label, cost) in &cost_items {
        let pct = if k.total_cost > 0.0 { cost / k.total_cost * 100.0 } else { 0.0 };
        lines.push(format!("  • {}: {} ({:.1}%)", label, fmt_cost(*cost), pct));
        data_points.push(DataPoint {
            label: label.to_string(),
            value: fmt_cost(*cost),
            context: Some(format!("{:.1}% of total", pct)),
        });
    }

    data_points.push(DataPoint {
        label: "Cost per unit delivered".into(),
        value: fmt_cost(k.cost_per_unit_delivered),
        context: Some(format!("{} units delivered", fmt_qty(k.total_delivered))),
    });

    // Find biggest cost driver
    let max_cost = cost_items.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
    lines.push(format!("\n📊 **Biggest cost driver: {}** at {}", max_cost.0, fmt_cost(max_cost.1)));

    // Insights
    if k.penalty_cost > 0.0 {
        lines.push(format!("\n⚠️ Penalty cost ({}) indicates unmet demand or safety stock violations. Reducing these could save {}.",
            fmt_cost(k.penalty_cost), fmt_cost(k.penalty_cost)));
    }

    // Find most expensive transport lanes
    let mut lane_costs: std::collections::HashMap<(String, String), f64> = std::collections::HashMap::new();
    for t in &result.transport_plan {
        *lane_costs.entry((t.from_location_id.clone(), t.to_location_id.clone())).or_insert(0.0) += t.cost;
    }
    if !lane_costs.is_empty() {
        let mut sorted: Vec<_> = lane_costs.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        let (ref lane, cost) = sorted[0];
        lines.push(format!("\n🚚 Most expensive route: {} → {} ({})",
            loc_name(model, &lane.0), loc_name(model, &lane.1), fmt_cost(*cost)));

        // Check if tariff is making it expensive
        if let Some(tl) = model.transport_lanes.iter().find(|tl| tl.from_location_id == lane.0 && tl.to_location_id == lane.1) {
            if tl.tariff_rate > 0.0 {
                lines.push(format!("  → 💡 This lane has a {:.0}% tariff — consider alternate sourcing", tl.tariff_rate * 100.0));
            }
        }
    }

    ExplainResponse {
        intent: "cost".into(),
        answer: lines.join("\n"),
        data_points,
        suggestions: vec![
            "Why is there unmet demand?".into(),
            "Which resources are bottlenecks?".into(),
        ],
    }
}

// ─── Production Explanation ─────────────────────────────────

fn explain_production(
    model: &SupplyChainModel,
    result: &SolverResult,
    location: Option<String>,
    product: Option<String>,
) -> ExplainResponse {
    let prods: Vec<_> = result.production_plan.iter().filter(|p| {
        let loc_match = location.as_ref().map_or(true, |l| &p.location_id == l);
        let prod_match = product.as_ref().map_or(true, |pr| &p.product_id == pr);
        loc_match && prod_match
    }).collect();

    if prods.is_empty() {
        return ExplainResponse {
            intent: "production".into(),
            answer: "No production planned for the specified criteria.".into(),
            data_points: vec![],
            suggestions: vec!["Check if sourcing rules and resources are configured".into()],
        };
    }

    let mut data_points = Vec::new();
    let mut lines = Vec::new();

    // Group by location
    let mut by_loc: std::collections::HashMap<String, Vec<&PlanEntry>> = std::collections::HashMap::new();
    for p in &prods {
        by_loc.entry(p.location_id.clone()).or_default().push(p);
    }

    for (lid, entries) in &by_loc {
        let l_name = loc_name(model, lid);
        let total_qty: f64 = entries.iter().map(|e| e.quantity).sum();
        let total_cost: f64 = entries.iter().map(|e| e.cost).sum();

        lines.push(format!("🏭 **{}**: {} units produced (cost: {})", l_name, fmt_qty(total_qty), fmt_cost(total_cost)));

        data_points.push(DataPoint {
            label: format!("Production at {}", l_name),
            value: fmt_qty(total_qty),
            context: Some(fmt_cost(total_cost)),
        });

        // By product at this location
        let mut by_prod: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for e in entries {
            *by_prod.entry(e.product_id.clone()).or_insert(0.0) += e.quantity;
        }
        for (pid, qty) in &by_prod {
            let yield_rate = model.get_yield_rate(pid, lid);
            lines.push(format!("  • {}: {} units (yield: {:.0}%)", prod_name(model, pid), fmt_qty(*qty), yield_rate * 100.0));
        }

        // Why here? Show cost comparison
        let res_at_loc: Vec<_> = model.resources.iter().filter(|r| r.location_id == *lid && r.active).collect();
        for res in &res_at_loc {
            lines.push(format!("  → Resource: {} — {}/unit, {} capacity/period",
                res_name(model, &res.id), fmt_cost(res.cost_per_unit), fmt_qty(res.capacity_per_period)));
        }
    }

    ExplainResponse {
        intent: "production".into(),
        answer: lines.join("\n"),
        data_points,
        suggestions: vec![
            "What are the bottleneck resources?".into(),
            "What is the cost breakdown?".into(),
        ],
    }
}

// ─── General Summary ────────────────────────────────────────

fn explain_general(
    model: &SupplyChainModel,
    result: &SolverResult,
) -> ExplainResponse {
    let k = &result.kpis;
    let status = match &result.status {
        SolveStatus::Optimal => "✅ Optimal",
        SolveStatus::Feasible => "🟡 Feasible (may not be optimal)",
        SolveStatus::Infeasible => "🔴 Infeasible",
        SolveStatus::TimedOut => "🟡 Timed Out (best solution found so far)",
        SolveStatus::Error(e) => return ExplainResponse {
            intent: "general".into(),
            answer: format!("❌ Solver error: {}", e),
            data_points: vec![],
            suggestions: vec!["Check your model configuration and try again".into()],
        },
    };

    let mut lines = Vec::new();
    lines.push(format!("📊 **Optimization Summary** — {}", status));
    lines.push(format!("Solved in {}ms | Total cost: {} | Fill rate: {:.1}%\n",
        result.solve_time_ms, fmt_cost(k.total_cost), k.demand_fulfillment_pct));

    lines.push(format!("📦 **Demand**: {} total → {} delivered, {} unmet",
        fmt_qty(k.total_demand), fmt_qty(k.total_delivered), fmt_qty(k.total_unmet)));

    lines.push(format!("💰 **Cost**: Production {} | Transport {} | Holding {} | Penalties {}",
        fmt_cost(k.production_cost), fmt_cost(k.transport_cost),
        fmt_cost(k.holding_cost), fmt_cost(k.penalty_cost)));

    lines.push(format!("⚙️ **Capacity**: {:.1}% avg utilization | {} bottleneck resource(s)",
        k.avg_capacity_utilization, k.num_bottleneck_resources));

    lines.push(format!("📦 **Inventory**: {} avg | {} peak",
        fmt_qty(k.avg_inventory), fmt_qty(k.peak_inventory)));

    // Actionable insights
    lines.push("\n**Insights:**".into());
    if k.total_unmet > 0.0 {
        lines.push(format!("  ⚠️ {:.0} units of unmet demand — ask me \"Why is there unmet demand?\" for details", k.total_unmet));
    }
    if k.num_bottleneck_resources > 0 {
        lines.push(format!("  🔴 {} bottleneck resources — ask me \"What are the bottlenecks?\"", k.num_bottleneck_resources));
    }
    if k.penalty_cost > k.total_cost * 0.1 {
        lines.push(format!("  💡 Penalties are {:.1}% of total cost — reducing unmet demand could significantly improve profitability",
            k.penalty_cost / k.total_cost * 100.0));
    }
    if k.demand_fulfillment_pct >= 99.9 && k.num_bottleneck_resources == 0 {
        lines.push("  🎯 Excellent! 100% demand fulfilled with no bottlenecks.".into());
    }

    let data_points = vec![
        DataPoint { label: "Status".into(), value: status.into(), context: Some(format!("{}ms", result.solve_time_ms)) },
        DataPoint { label: "Demand Fulfillment".into(), value: format!("{:.1}%", k.demand_fulfillment_pct), context: None },
        DataPoint { label: "Total Cost".into(), value: fmt_cost(k.total_cost), context: None },
        DataPoint { label: "Units Delivered".into(), value: fmt_qty(k.total_delivered), context: Some(format!("of {}", fmt_qty(k.total_demand))) },
        DataPoint { label: "Avg Capacity Utilization".into(), value: format!("{:.1}%", k.avg_capacity_utilization), context: None },
        DataPoint { label: "Bottlenecks".into(), value: k.num_bottleneck_resources.to_string(), context: None },
    ];

    ExplainResponse {
        intent: "general".into(),
        answer: lines.join("\n"),
        data_points,
        suggestions: vec![
            "Why is there unmet demand?".into(),
            "What is driving the cost?".into(),
            "Which resources are bottlenecks?".into(),
            "Why is inventory building up?".into(),
        ],
    }
}
