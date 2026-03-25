// OptiFlow — Pre-Solve Validator
// Catches common mistakes BEFORE sending to the solver.

use crate::models::*;

pub fn validate_model(model: &SupplyChainModel) -> Vec<ValidationMessage> {
    let mut messages = Vec::new();

    // ─── Check: At least one location ───
    if model.locations.is_empty() {
        messages.push(ValidationMessage {
            severity: ValidationSeverity::Error,
            category: "Locations".into(),
            message: "No locations defined. You need at least one plant and one customer.".into(),
            field: None,
            suggestion: Some("Add locations in the Master Data → Locations tab.".into()),
        });
    }

    // ─── Check: At least one product ───
    if model.products.is_empty() {
        messages.push(ValidationMessage {
            severity: ValidationSeverity::Error,
            category: "Products".into(),
            message: "No products defined.".into(),
            field: None,
            suggestion: Some("Add products in the Master Data → Products tab.".into()),
        });
    }

    // ─── Check: At least one demand ───
    if model.demands.is_empty() {
        messages.push(ValidationMessage {
            severity: ValidationSeverity::Error,
            category: "Demand".into(),
            message: "No demand records. The optimizer has nothing to plan for.".into(),
            field: None,
            suggestion: Some("Import or create demand data.".into()),
        });
    }

    // ─── Check: Non-delivery costs ───
    for demand in &model.demands {
        if demand.non_delivery_cost <= 0.0 {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Warning,
                category: "Costs".into(),
                message: format!(
                    "Non-delivery cost is ${:.2} for product '{}' at '{}' period {}. \
                     Optimizer won't prioritize this delivery.",
                    demand.non_delivery_cost, demand.product_id, demand.location_id, demand.period
                ),
                field: Some("non_delivery_cost".into()),
                suggestion: Some("Set non-delivery cost higher than production + transport cost.".into()),
            });
        }
    }

    // ─── Check: Product-Location records exist ───
    if model.product_locations.is_empty() && !model.demands.is_empty() {
        messages.push(ValidationMessage {
            severity: ValidationSeverity::Warning,
            category: "Product-Location".into(),
            message: "No Product-Location records defined. Inventory tracking, safety stock, and holding costs won't be modeled.".into(),
            field: None,
            suggestion: Some("Add Product-Location records to enable multi-period inventory planning.".into()),
        });
    }

    // ─── Check: Safety stock with zero violation cost ───
    for pl in &model.product_locations {
        if pl.safety_stock > 0.0 && pl.safety_stock_violation_cost <= 0.0 {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Warning,
                category: "Product-Location".into(),
                message: format!(
                    "Safety stock is {} for '{}' at '{}' but violation cost is $0. Safety stock won't be enforced.",
                    pl.safety_stock, pl.product_id, pl.location_id
                ),
                field: Some("safety_stock_violation_cost".into()),
                suggestion: Some("Set safety stock violation cost > 0 to enforce the target.".into()),
            });
        }
    }

    // ─── Check: Product-Location initial inventory for demand locations ───
    for demand in &model.demands {
        if demand.period == 0 {
            if let Some(pl) = model.get_product_location(&demand.product_id, &demand.location_id) {
                if pl.initial_inventory > 0.0 && pl.initial_inventory >= demand.quantity {
                    messages.push(ValidationMessage {
                        severity: ValidationSeverity::Info,
                        category: "Inventory".into(),
                        message: format!(
                            "Initial inventory ({}) at '{}' covers first-period demand ({}) for '{}'.",
                            pl.initial_inventory, demand.location_id, demand.quantity, demand.product_id
                        ),
                        field: None,
                        suggestion: None,
                    });
                }
            }
        }
    }

    // ─── Check: Demands have valid sourcing paths ───
    let customer_locations: Vec<&str> = model
        .demands
        .iter()
        .map(|d| d.location_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for cust_loc in &customer_locations {
        let has_supply_path = model.sourcing_rules.iter().any(|sr| {
            (sr.sourcing_type == SourcingType::CustomerAlloc
                || sr.sourcing_type == SourcingType::Transport)
                && sr.to_location_id.as_deref() == Some(cust_loc)
                && sr.active
        });

        let has_transport_lane = model.transport_lanes.iter().any(|tl| {
            tl.to_location_id == *cust_loc && tl.active
        });

        let has_local_production = model.sourcing_rules.iter().any(|sr| {
            sr.sourcing_type == SourcingType::Production
                && sr.from_location_id == *cust_loc
                && sr.active
        });

        if !has_supply_path && !has_transport_lane && !has_local_production {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Error,
                category: "Sourcing".into(),
                message: format!(
                    "Location '{}' has demand but no supply source. Model will be infeasible.",
                    cust_loc
                ),
                field: None,
                suggestion: Some("Create a transport lane or sourcing rule that delivers to this location.".into()),
            });
        }
    }

    // ─── Check: Resources have capacity ───
    for resource in &model.resources {
        if resource.capacity_per_period <= 0.0 && resource.active {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Warning,
                category: "Resources".into(),
                message: format!(
                    "Resource '{}' at '{}' has zero capacity. No production can happen here.",
                    resource.name, resource.location_id
                ),
                field: Some("capacity_per_period".into()),
                suggestion: Some("Set a positive capacity or deactivate this resource.".into()),
            });
        }
    }

    // ─── Check: Production rules exist for products ───
    let products_with_demand: Vec<&str> = model
        .demands
        .iter()
        .map(|d| d.product_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for prod_id in &products_with_demand {
        let has_production = model.sourcing_rules.iter().any(|sr| {
            sr.sourcing_type == SourcingType::Production
                && sr.product_id == *prod_id
                && sr.active
        });

        let has_procurement = model.sourcing_rules.iter().any(|sr| {
            sr.sourcing_type == SourcingType::Procurement
                && sr.product_id == *prod_id
                && sr.active
        });

        if !has_production && !has_procurement {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Error,
                category: "Sourcing".into(),
                message: format!(
                    "Product '{}' has demand but no production or procurement rule.",
                    prod_id
                ),
                field: None,
                suggestion: Some(
                    "Create a Production rule (to make it) or Procurement rule (to buy it).".into(),
                ),
            });
        }
    }

    // ─── Check: Yield rates ───
    for product in &model.products {
        if product.yield_rate < 0.5 {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Warning,
                category: "Products".into(),
                message: format!(
                    "Product '{}' has {:.0}% yield — over half is scrap. Verify this is correct.",
                    product.name, product.yield_rate * 100.0
                ),
                field: Some("yield_rate".into()),
                suggestion: None,
            });
        }
    }

    // ─── Check: BOM completeness ───
    for bom in &model.bom_entries {
        let input_exists = model.products.iter().any(|p| p.id == bom.input_product_id);
        if !input_exists {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Error,
                category: "BOM".into(),
                message: format!(
                    "BOM for '{}' references input '{}' which doesn't exist.",
                    bom.output_product_id, bom.input_product_id
                ),
                field: None,
                suggestion: Some("Add the missing product or correct the BOM entry.".into()),
            });
        }
    }

    // ─── Check: Product-Resource coherence ───
    for pr in &model.product_resources {
        if pr.consumption_rate <= 0.0 {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Warning,
                category: "Product-Resource".into(),
                message: format!(
                    "Consumption rate is 0 for product '{}' on resource '{}'. This means unlimited production.",
                    pr.product_id, pr.resource_id
                ),
                field: Some("consumption_rate".into()),
                suggestion: Some("Set a positive consumption rate (e.g., 1.0 = 1 resource unit per product unit).".into()),
            });
        }
    }

    // ─── Check: Lead time feasibility ───
    for tl in &model.transport_lanes {
        if tl.lead_time_periods > 0 && tl.lead_time_periods >= model.num_periods {
            messages.push(ValidationMessage {
                severity: ValidationSeverity::Warning,
                category: "Transport".into(),
                message: format!(
                    "Transport from '{}' to '{}' has lead time of {} periods, but planning horizon is only {} periods. Nothing shipped will arrive in time.",
                    tl.from_location_id, tl.to_location_id, tl.lead_time_periods, model.num_periods
                ),
                field: Some("lead_time_periods".into()),
                suggestion: Some("Increase planning horizon or reduce lead time.".into()),
            });
        }
    }

    // ─── Summary info ───
    let pl_count = model.product_locations.len();
    let pr_count = model.product_resources.len();
    messages.push(ValidationMessage {
        severity: ValidationSeverity::Info,
        category: "Summary".into(),
        message: format!(
            "Model '{}': {} locations, {} products, {} resources, {} transport lanes, {} demands, \
             {} product-locations, {} product-resources across {} periods.",
            model.name, model.locations.len(), model.products.len(),
            model.resources.len(), model.transport_lanes.len(), model.demands.len(),
            pl_count, pr_count, model.num_periods
        ),
        field: None,
        suggestion: None,
    });

    messages
}
