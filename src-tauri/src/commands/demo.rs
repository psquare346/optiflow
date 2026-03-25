// OptiFlow — Demo data loader
// Creates a realistic semiconductor supply chain for testing.

use crate::models::*;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn load_demo_data(state: State<AppState>) -> String {
    let mut model = state.model.lock().unwrap();
    *model = SupplyChainModel::new("Semiconductor Demo", 8);

    // ─── Locations ───
    model.locations.push(Location::new("LOC-TW-FAB", "Taiwan Fab (TSMC-style)", LocationType::Plant, "Taiwan", "APAC"));
    model.locations.push(Location::new("LOC-TX-FAB", "Texas Assembly", LocationType::Plant, "USA", "Americas"));
    model.locations.push(Location::new("LOC-DE-WH", "Germany Warehouse", LocationType::Warehouse, "Germany", "EMEA"));
    model.locations.push(Location::new("LOC-BMW", "BMW Munich", LocationType::Customer, "Germany", "EMEA"));
    model.locations.push(Location::new("LOC-TESLA", "Tesla Austin", LocationType::Customer, "USA", "Americas"));

    // ─── Products ───
    model.products.push(Product::new("PROD-A14", "A14 Processor", ProductType::FinishedGood, "EA", 0.92));
    model.products.push(Product::new("PROD-M2", "M2 Chip", ProductType::FinishedGood, "EA", 0.88));
    model.products.push(Product::new("PROD-WAFER", "300mm Silicon Wafer", ProductType::RawMaterial, "EA", 1.0));

    // ─── Resources ───
    model.resources.push(Resource::new("RES-TW-LINE1", "Taiwan Fab Line 1", "LOC-TW-FAB", 50000.0, 12.0));
    model.resources.push(Resource::new("RES-TX-LINE1", "Texas Fab Line 1", "LOC-TX-FAB", 30000.0, 15.0));

    // ─── Suppliers (NEW) ───
    model.suppliers.push(Supplier::new("SUP-SUMCO", "SUMCO Corp", "Japan"));
    model.suppliers.push(Supplier::new("SUP-SHIN", "Shin-Etsu Chemical", "Japan"));

    // ─── Customers (NEW) ───
    let mut cust_bmw = Customer::new("CUST-BMW", "BMW AG", "Germany");
    cust_bmw.priority = 1;
    cust_bmw.service_level_target = 0.98;
    model.customers.push(cust_bmw);

    let mut cust_tesla = Customer::new("CUST-TESLA", "Tesla Inc", "USA");
    cust_tesla.priority = 1;
    cust_tesla.service_level_target = 0.95;
    model.customers.push(cust_tesla);

    // ─── Product-Location relationships ───
    let mut pl_tw_a14 = ProductLocation::new("PROD-A14", "LOC-TW-FAB");
    pl_tw_a14.can_produce = true;
    pl_tw_a14.initial_inventory = 5000.0;
    pl_tw_a14.safety_stock = 2000.0;
    pl_tw_a14.holding_cost_per_unit = 0.50;
    pl_tw_a14.safety_stock_violation_cost = 15.0;
    model.product_locations.push(pl_tw_a14);

    let mut pl_tw_m2 = ProductLocation::new("PROD-M2", "LOC-TW-FAB");
    pl_tw_m2.can_produce = true;
    pl_tw_m2.initial_inventory = 3000.0;
    pl_tw_m2.safety_stock = 1500.0;
    pl_tw_m2.holding_cost_per_unit = 0.60;
    pl_tw_m2.safety_stock_violation_cost = 15.0;
    model.product_locations.push(pl_tw_m2);

    let mut pl_tw_wafer = ProductLocation::new("PROD-WAFER", "LOC-TW-FAB");
    pl_tw_wafer.can_procure = true;
    pl_tw_wafer.procurement_cost = 3.0;
    pl_tw_wafer.initial_inventory = 80000.0;
    pl_tw_wafer.holding_cost_per_unit = 0.10;
    model.product_locations.push(pl_tw_wafer);

    let mut pl_tx_a14 = ProductLocation::new("PROD-A14", "LOC-TX-FAB");
    pl_tx_a14.can_produce = true;
    pl_tx_a14.initial_inventory = 2000.0;
    pl_tx_a14.safety_stock = 1000.0;
    pl_tx_a14.holding_cost_per_unit = 0.55;
    pl_tx_a14.yield_rate_override = Some(0.90);
    model.product_locations.push(pl_tx_a14);

    let mut pl_de_a14 = ProductLocation::new("PROD-A14", "LOC-DE-WH");
    pl_de_a14.initial_inventory = 8000.0;
    pl_de_a14.safety_stock = 3000.0;
    pl_de_a14.max_stock = 50000.0;
    pl_de_a14.holding_cost_per_unit = 0.40;
    model.product_locations.push(pl_de_a14);

    let mut pl_de_m2 = ProductLocation::new("PROD-M2", "LOC-DE-WH");
    pl_de_m2.initial_inventory = 4000.0;
    pl_de_m2.safety_stock = 2000.0;
    pl_de_m2.max_stock = 30000.0;
    pl_de_m2.holding_cost_per_unit = 0.45;
    model.product_locations.push(pl_de_m2);

    model.product_locations.push(ProductLocation::new("PROD-A14", "LOC-BMW"));
    model.product_locations.push(ProductLocation::new("PROD-M2", "LOC-BMW"));
    model.product_locations.push(ProductLocation::new("PROD-A14", "LOC-TESLA"));

    // ─── Product-Resource relationships ───
    let mut pr_tw_a14 = ProductResource::new("PROD-A14", "RES-TW-LINE1", "LOC-TW-FAB");
    pr_tw_a14.consumption_rate = 1.0;
    model.product_resources.push(pr_tw_a14);

    let mut pr_tw_m2 = ProductResource::new("PROD-M2", "RES-TW-LINE1", "LOC-TW-FAB");
    pr_tw_m2.consumption_rate = 1.2;
    model.product_resources.push(pr_tw_m2);

    let mut pr_tx_a14 = ProductResource::new("PROD-A14", "RES-TX-LINE1", "LOC-TX-FAB");
    pr_tx_a14.consumption_rate = 1.0;
    model.product_resources.push(pr_tx_a14);

    // ─── BOM: Wafer → Chips ───
    model.bom_entries.push(BomEntry {
        output_product_id: "PROD-A14".into(),
        input_product_id: "PROD-WAFER".into(),
        quantity_per: 0.5,
        yield_rate: 1.0,
        location_id: None,
    });
    model.bom_entries.push(BomEntry {
        output_product_id: "PROD-M2".into(),
        input_product_id: "PROD-WAFER".into(),
        quantity_per: 0.8,
        yield_rate: 1.0,
        location_id: None,
    });

    // ─── Transport Lanes ───
    let mut tl1 = TransportLane::new("LOC-TW-FAB", "LOC-DE-WH", TransportMode::Ocean, 0.85, 21);
    tl1.lead_time_periods = 1;
    model.transport_lanes.push(tl1);

    let mut tl2 = TransportLane::new("LOC-TW-FAB", "LOC-TESLA", TransportMode::Ocean, 1.20, 18);
    tl2.lead_time_periods = 1;
    tl2.tariff_rate = 0.25;
    model.transport_lanes.push(tl2);

    let mut tl3 = TransportLane::new("LOC-TX-FAB", "LOC-TESLA", TransportMode::Truck, 0.30, 2);
    tl3.lead_time_periods = 0;
    model.transport_lanes.push(tl3);

    let mut tl4 = TransportLane::new("LOC-TX-FAB", "LOC-DE-WH", TransportMode::Ocean, 1.50, 14);
    tl4.lead_time_periods = 1;
    model.transport_lanes.push(tl4);

    let mut tl5 = TransportLane::new("LOC-DE-WH", "LOC-BMW", TransportMode::Truck, 0.15, 1);
    tl5.lead_time_periods = 0;
    model.transport_lanes.push(tl5);

    // ─── Sourcing Rules ───
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Production,
        product_id: "PROD-A14".into(), from_location_id: "LOC-TW-FAB".into(),
        to_location_id: None, resource_id: Some("RES-TW-LINE1".into()),
        priority: 1, quota_percentage: 0.7, min_lot_size: 1000.0, max_lot_size: 50000.0, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Production,
        product_id: "PROD-M2".into(), from_location_id: "LOC-TW-FAB".into(),
        to_location_id: None, resource_id: Some("RES-TW-LINE1".into()),
        priority: 1, quota_percentage: 1.0, min_lot_size: 500.0, max_lot_size: 30000.0, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Production,
        product_id: "PROD-A14".into(), from_location_id: "LOC-TX-FAB".into(),
        to_location_id: None, resource_id: Some("RES-TX-LINE1".into()),
        priority: 2, quota_percentage: 0.3, min_lot_size: 500.0, max_lot_size: 30000.0, active: true,
    });

    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Transport,
        product_id: "PROD-A14".into(), from_location_id: "LOC-TW-FAB".into(),
        to_location_id: Some("LOC-DE-WH".into()), resource_id: None,
        priority: 1, quota_percentage: 1.0, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Transport,
        product_id: "PROD-M2".into(), from_location_id: "LOC-TW-FAB".into(),
        to_location_id: Some("LOC-DE-WH".into()), resource_id: None,
        priority: 1, quota_percentage: 1.0, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Transport,
        product_id: "PROD-A14".into(), from_location_id: "LOC-TW-FAB".into(),
        to_location_id: Some("LOC-TESLA".into()), resource_id: None,
        priority: 2, quota_percentage: 1.0, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Transport,
        product_id: "PROD-A14".into(), from_location_id: "LOC-TX-FAB".into(),
        to_location_id: Some("LOC-TESLA".into()), resource_id: None,
        priority: 1, quota_percentage: 1.0, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Transport,
        product_id: "PROD-A14".into(), from_location_id: "LOC-TX-FAB".into(),
        to_location_id: Some("LOC-DE-WH".into()), resource_id: None,
        priority: 2, quota_percentage: 1.0, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Transport,
        product_id: "PROD-A14".into(), from_location_id: "LOC-DE-WH".into(),
        to_location_id: Some("LOC-BMW".into()), resource_id: None,
        priority: 1, quota_percentage: 1.0, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    });
    model.sourcing_rules.push(SourcingRule {
        id: Uuid::new_v4().to_string(), sourcing_type: SourcingType::Transport,
        product_id: "PROD-M2".into(), from_location_id: "LOC-DE-WH".into(),
        to_location_id: Some("LOC-BMW".into()), resource_id: None,
        priority: 1, quota_percentage: 1.0, min_lot_size: 0.0, max_lot_size: f64::MAX, active: true,
    });

    // ─── Planned Receipts (open POs, in-transit, production orders) ───
    // These are firm supply that exists regardless of optimizer decisions.
    // Critical for frozen zone: only these + initial inventory serve demand.
    model.planned_receipts.push(PlannedReceipt {
        id: Uuid::new_v4().to_string(), product_id: "PROD-WAFER".into(),
        location_id: "LOC-TW-FAB".into(), period: 0, quantity: 40000.0,
        receipt_type: ReceiptType::PurchaseOrder, source: Some("SUP-SUMCO".into()), is_firm: true,
    });
    model.planned_receipts.push(PlannedReceipt {
        id: Uuid::new_v4().to_string(), product_id: "PROD-WAFER".into(),
        location_id: "LOC-TW-FAB".into(), period: 2, quantity: 40000.0,
        receipt_type: ReceiptType::PurchaseOrder, source: Some("SUP-SHIN".into()), is_firm: true,
    });
    model.planned_receipts.push(PlannedReceipt {
        id: Uuid::new_v4().to_string(), product_id: "PROD-A14".into(),
        location_id: "LOC-DE-WH".into(), period: 1, quantity: 12000.0,
        receipt_type: ReceiptType::InTransit, source: Some("LOC-TW-FAB".into()), is_firm: true,
    });
    model.planned_receipts.push(PlannedReceipt {
        id: Uuid::new_v4().to_string(), product_id: "PROD-M2".into(),
        location_id: "LOC-DE-WH".into(), period: 1, quantity: 8000.0,
        receipt_type: ReceiptType::InTransit, source: Some("LOC-TW-FAB".into()), is_firm: true,
    });
    model.planned_receipts.push(PlannedReceipt {
        id: Uuid::new_v4().to_string(), product_id: "PROD-A14".into(),
        location_id: "LOC-TX-FAB".into(), period: 0, quantity: 5000.0,
        receipt_type: ReceiptType::ProductionOrder, source: None, is_firm: true,
    });
    model.planned_receipts.push(PlannedReceipt {
        id: Uuid::new_v4().to_string(), product_id: "PROD-A14".into(),
        location_id: "LOC-TESLA".into(), period: 0, quantity: 10000.0,
        receipt_type: ReceiptType::InTransit, source: Some("LOC-TX-FAB".into()), is_firm: true,
    });

    // ─── Product Prices (for MaxProfit objective) ───
    model.product_prices.push(ProductPrice {
        product_id: "PROD-A14".into(), location_id: None, customer_id: None, period: None,
        price_per_unit: 45.0,
    });
    model.product_prices.push(ProductPrice {
        product_id: "PROD-M2".into(), location_id: None, customer_id: None, period: None,
        price_per_unit: 55.0,
    });

    // ─── Demand (8 periods — realistic seasonal curve) ───
    let demand_data = vec![
        // BMW: A14 — steady with mid-horizon peak
        ("PROD-A14", "LOC-BMW", "CUST-BMW",
         vec![8000.0, 9000.0, 10000.0, 12000.0, 14000.0, 12000.0, 10000.0, 9000.0]),
        // BMW: M2 — gradual ramp
        ("PROD-M2",  "LOC-BMW", "CUST-BMW",
         vec![5000.0, 5500.0, 6000.0, 7000.0, 8000.0, 7000.0, 6000.0, 5000.0]),
        // Tesla: A14 — high volume with demand surge in periods 3-5
        ("PROD-A14", "LOC-TESLA", "CUST-TESLA",
         vec![15000.0, 16000.0, 18000.0, 22000.0, 25000.0, 20000.0, 17000.0, 15000.0]),
    ];

    for (prod, loc, cust, quantities) in demand_data {
        for (period, qty) in quantities.iter().enumerate() {
            // First 2 periods are Sales Orders (firm); rest are Forecasts
            let (d_type, firm) = if period < 2 {
                (DemandType::SalesOrder, true)
            } else {
                (DemandType::Forecast, false)
            };
            model.demands.push(Demand {
                id: Uuid::new_v4().to_string(),
                product_id: prod.into(),
                location_id: loc.into(),
                period: period as u32,
                quantity: *qty,
                priority: 1,
                non_delivery_cost: 50.0,
                late_delivery_cost: 5.0,
                demand_type: d_type,
                customer_id: Some(cust.into()),
                is_firm: firm,
            });
        }
    }

    format!(
        "Demo loaded: {} locations, {} products, {} resources, {} suppliers, {} customers, \
         {} product-locations, {} demands, {} receipts, {} prices — {} periods",
        model.locations.len(), model.products.len(), model.resources.len(),
        model.suppliers.len(), model.customers.len(),
        model.product_locations.len(), model.demands.len(),
        model.planned_receipts.len(), model.product_prices.len(), model.num_periods
    )
}
