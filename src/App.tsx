import { useState, useEffect, useCallback, useRef } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import "./index.css";

// ─── Safe invoke wrapper (handles running outside Tauri WebView) ──

const isTauri = typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;

// Commands that only read data (use GET), everything else uses POST
const GET_COMMANDS = new Set([
  "get_locations", "get_products", "get_resources", "get_transport_lanes",
  "get_demands", "get_suppliers", "get_customers", "get_planned_receipts",
  "get_product_prices", "get_product_locations", "get_product_resources",
  "get_bom_entries", "get_sourcing_rules", "get_aliases", "validate_model",
  "get_last_result",
]);

interface ExplainDataPoint { label: string; value: string; context: string | null; }
interface ExplainResponse { intent: string; answer: string; data_points: ExplainDataPoint[]; suggestions: string[]; }

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    return tauriInvoke<T>(cmd, args);
  }
  // HTTP fallback for Docker / cloud deployment
  const isGet = GET_COMMANDS.has(cmd);
  const url = `/api/${cmd}`;
  const res = await fetch(url, {
    method: isGet ? "GET" : "POST",
    headers: isGet ? {} : { "Content-Type": "application/json" },
    body: isGet ? undefined : JSON.stringify(args ?? {}),
  });
  if (!res.ok) throw new Error(`API error: ${res.status} ${res.statusText}`);
  // Handle empty responses (204 or empty body)
  const text = await res.text();
  if (!text) return undefined as unknown as T;
  return JSON.parse(text) as T;
}

// ─── Types ──────────────────────────────────────────────────

interface Location {
  id: string; name: string; location_type: string; country: string; region: string;
  latitude: number | null; longitude: number | null; active: boolean;
}

interface Product {
  id: string; name: string; product_type: string; unit_of_measure: string;
  yield_rate: number; weight_kg: number; volume_m3: number; shelf_life_days: number | null; active: boolean;
}

interface Resource {
  id: string; name: string; location_id: string; capacity_type: string;
  capacity_per_period: number; cost_per_unit: number; setup_cost: number; active: boolean;
}

interface TransportLane {
  id: string; from_location_id: string; to_location_id: string; mode: string;
  cost_per_unit: number; fixed_cost_per_shipment: number;
  lead_time_periods: number; lead_time_days: number;
  min_lot_size: number; max_lot_size: number; tariff_rate: number; co2_per_unit: number; active: boolean;
}

interface Demand {
  id: string; product_id: string; location_id: string; period: number;
  quantity: number; priority: number; non_delivery_cost: number; late_delivery_cost: number;
  demand_type: string; customer_id: string | null; is_firm: boolean;
}

interface Supplier {
  id: string; name: string; country: string; lead_time_days: number;
  capacity_per_period: number; quality_rating: number; active: boolean;
}

interface Customer {
  id: string; name: string; priority: number; service_level_target: number;
  country: string; active: boolean;
}

interface PlannedReceipt {
  id: string; product_id: string; location_id: string; period: number;
  quantity: number; receipt_type: string; source: string | null; is_firm: boolean;
}

interface ProductPrice {
  product_id: string; location_id: string | null; customer_id: string | null;
  period: number | null; price_per_unit: number;
}

interface ProductLocation {
  product_id: string; location_id: string;
  initial_inventory: number; safety_stock: number; max_stock: number;
  holding_cost_per_unit: number; safety_stock_violation_cost: number; max_stock_violation_cost: number;
  can_produce: boolean; yield_rate_override: number | null; production_lead_time: number;
  can_procure: boolean; procurement_cost: number; procurement_lead_time: number;
  min_lot_size: number; max_lot_size: number; can_store: boolean; active: boolean;
}

interface ProductResource {
  product_id: string; resource_id: string; location_id: string;
  consumption_rate: number; setup_time: number; changeover_cost: number;
  production_rate: number; version_id: string; priority: number; active: boolean;
}

interface BomEntry {
  output_product_id: string; input_product_id: string; quantity_per: number; yield_rate: number;
}

interface ValidationMessage { severity: string; category: string; message: string; field: string | null; suggestion: string | null; }

interface SolverConfig {
  objective: string; time_limit_seconds: number; optimality_gap: number;
  frozen_periods: number; firm_periods: number; firm_change_penalty: number;
}

interface DashboardKpis {
  demand_fulfillment_pct: number; total_delivered: number; total_demand: number; total_unmet: number;
  total_cost: number; production_cost: number; transport_cost: number; holding_cost: number; penalty_cost: number;
  cost_per_unit_delivered: number; avg_capacity_utilization: number; num_bottleneck_resources: number;
  avg_inventory: number; peak_inventory: number;
}

interface PlanEntry { product_id: string; location_id: string; resource_id: string; period: number; quantity: number; cost: number; }
interface TransportPlanEntry { product_id: string; from_location_id: string; to_location_id: string; period: number; quantity: number; cost: number; mode: string; }
interface UnmetDemandEntry { demand_id: string; product_id: string; location_id: string; period: number; unmet_quantity: number; penalty_cost: number; reason: string; }
interface CapacityUtilEntry { resource_id: string; location_id: string; period: number; used: number; available: number; utilization_pct: number; }
interface InventoryEntry { product_id: string; location_id: string; period: number; quantity: number; holding_cost: number; safety_stock_delta: number; }

interface SolverResult {
  status: string | { Error: string }; solve_time_ms: number; objective_value: number;
  production_plan: PlanEntry[]; transport_plan: TransportPlanEntry[];
  inventory_plan: InventoryEntry[]; unmet_demand: UnmetDemandEntry[];
  capacity_utilization: CapacityUtilEntry[]; kpis: DashboardKpis;
}

interface AliasMap { aliases: Record<string, string>; }

// ─── Navigation ─────────────────────────────────────────────

type Page = "dashboard" | "locations" | "products" | "resources" | "transport" | "demands"
  | "suppliers" | "customers" | "planned_receipts" | "pricing"
  | "product_locations" | "product_resources" | "bom"
  | "solver" | "results" | "aliases" | "guide";

const NAV_ITEMS: { section: string; items: { id: Page; icon: string; label: string }[] }[] = [
  { section: "Overview", items: [{ id: "dashboard", icon: "📊", label: "Dashboard" }] },
  {
    section: "Master Data", items: [
      { id: "locations", icon: "🏭", label: "Locations" },
      { id: "products", icon: "📦", label: "Products" },
      { id: "resources", icon: "⚙️", label: "Resources" },
      { id: "transport", icon: "🚚", label: "Transport Lanes" },
      { id: "suppliers", icon: "🏢", label: "Suppliers" },
      { id: "customers", icon: "👤", label: "Customers" },
    ],
  },
  {
    section: "Transactions", items: [
      { id: "demands", icon: "📋", label: "Demand" },
      { id: "planned_receipts", icon: "📥", label: "Planned Receipts" },
      { id: "pricing", icon: "💰", label: "Pricing" },
    ],
  },
  {
    section: "Relationships", items: [
      { id: "product_locations", icon: "📍", label: "Product ↔ Location" },
      { id: "product_resources", icon: "🔗", label: "Product ↔ Resource" },
      { id: "bom", icon: "🧬", label: "Bill of Materials" },
    ],
  },
  {
    section: "Optimizer", items: [
      { id: "solver", icon: "🧮", label: "Run Optimizer" },
      { id: "results", icon: "📈", label: "Results" },
    ],
  },
  {
    section: "Settings", items: [
      { id: "aliases", icon: "🏷️", label: "Field Names" },
      { id: "guide", icon: "📖", label: "Documentation" },
    ],
  },
];

// ─── App ────────────────────────────────────────────────────

function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [locations, setLocations] = useState<Location[]>([]);
  const [products, setProducts] = useState<Product[]>([]);
  const [resources, setResources] = useState<Resource[]>([]);
  const [transportLanes, setTransportLanes] = useState<TransportLane[]>([]);
  const [demands, setDemands] = useState<Demand[]>([]);
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [plannedReceipts, setPlannedReceipts] = useState<PlannedReceipt[]>([]);
  const [productPrices, setProductPrices] = useState<ProductPrice[]>([]);
  const [productLocations, setProductLocations] = useState<ProductLocation[]>([]);
  const [productResources, setProductResources] = useState<ProductResource[]>([]);
  const [bomEntries, setBomEntries] = useState<BomEntry[]>([]);
  const [validation, setValidation] = useState<ValidationMessage[]>([]);
  const [solverResult, setSolverResult] = useState<SolverResult | null>(null);
  const [solving, setSolving] = useState(false);
  const [aliases, setAliases] = useState<AliasMap>({ aliases: {} });
  const [statusMessage, setStatusMessage] = useState("Ready");
  const [solverConfig, setSolverConfig] = useState<SolverConfig>({
    objective: "MinimizeCost", time_limit_seconds: 300, optimality_gap: 0.01,
    frozen_periods: 0, firm_periods: 0, firm_change_penalty: 100,
  });

  const refreshData = useCallback(async () => {
    try {
      const [locs, prods, res, lanes, dems, sups, custs, receipts, prices, pls, prs, boms, als] = await Promise.all([
        invoke<Location[]>("get_locations"),
        invoke<Product[]>("get_products"),
        invoke<Resource[]>("get_resources"),
        invoke<TransportLane[]>("get_transport_lanes"),
        invoke<Demand[]>("get_demands"),
        invoke<Supplier[]>("get_suppliers"),
        invoke<Customer[]>("get_customers"),
        invoke<PlannedReceipt[]>("get_planned_receipts"),
        invoke<ProductPrice[]>("get_product_prices"),
        invoke<ProductLocation[]>("get_product_locations"),
        invoke<ProductResource[]>("get_product_resources"),
        invoke<BomEntry[]>("get_bom_entries"),
        invoke<AliasMap>("get_aliases"),
      ]);
      setLocations(locs); setProducts(prods); setResources(res);
      setTransportLanes(lanes); setDemands(dems);
      setSuppliers(sups); setCustomers(custs);
      setPlannedReceipts(receipts); setProductPrices(prices);
      setProductLocations(pls); setProductResources(prs);
      setBomEntries(boms); setAliases(als);
    } catch (e) {
      console.error("Refresh failed:", e);
    }
  }, []);

  useEffect(() => { refreshData(); }, [refreshData]);

  const getLabel = (key: string): string => aliases.aliases[key] || key.replace(/_/g, " ");

  const loadDemo = async () => {
    try {
      const msg = await invoke<string>("load_demo_data");
      setStatusMessage(msg);
      await refreshData();
      setPage("dashboard");
    } catch (e) { setStatusMessage(`Error: ${e}`); }
  };

  const runValidation = async () => {
    try { setValidation(await invoke<ValidationMessage[]>("validate_model")); }
    catch (e) { console.error("Validation error:", e); }
  };

  const runSolver = async () => {
    setSolving(true); setStatusMessage("Solving...");
    try {
      const result = await invoke<SolverResult>("run_optimizer", { config: solverConfig });
      setSolverResult(result);
      const s = typeof result.status === "string" ? result.status : "Error";
      setStatusMessage(`${s} in ${result.solve_time_ms}ms — Cost: $${result.kpis.total_cost.toLocaleString()} | Fill: ${result.kpis.demand_fulfillment_pct.toFixed(1)}%`);
      setPage("results");
    } catch (e) { setStatusMessage(`Solver error: ${e}`); }
    finally { setSolving(false); }
  };

  const fmt = (n: number, d = 0): string => n.toLocaleString(undefined, { maximumFractionDigits: d });
  const fmtPct = (n: number): string => `${n.toFixed(1)}%`;
  const fmtCost = (n: number): string => `$${fmt(n, 2)}`;

  return (
    <div className="app-layout">
      {/* Sidebar */}
      <nav className="sidebar">
        <div className="sidebar-header">
          <div className="sidebar-logo">OF</div>
          <div><div className="sidebar-title">OptiFlow</div><div className="sidebar-subtitle">Supply Chain Optimizer</div></div>
        </div>
        <div className="sidebar-nav">
          {NAV_ITEMS.map((section) => (
            <div className="nav-section" key={section.section}>
              <div className="nav-section-title">{section.section}</div>
              {section.items.map((item) => (
                <div key={item.id} className={`nav-item ${page === item.id ? "active" : ""}`}
                  onClick={() => setPage(item.id)}>
                  <span className="nav-icon">{item.icon}</span>{item.label}
                </div>
              ))}
            </div>
          ))}
        </div>
        <div style={{ padding: "12px 8px", borderTop: "1px solid var(--border-subtle)" }}>
          <button className="btn btn-primary" style={{ width: "100%" }} onClick={loadDemo}>⚡ Load Demo Data</button>
        </div>
      </nav>

      {/* Main Content */}
      <div className="main-content">
        {!isTauri && (
          <div style={{ background: "linear-gradient(90deg, #6366f1, #8b5cf6)", color: "#fff", padding: "10px 20px", fontSize: 13, fontWeight: 500, textAlign: "center" }}>
            🌐 Running in <strong>web mode</strong> — connected to the OptiFlow API server
          </div>
        )}
        <header className="main-header">
          <h1>
            {page === "dashboard" && "📊 Dashboard"}
            {page === "locations" && `🏭 ${getLabel("locations")}`}
            {page === "products" && `📦 ${getLabel("products")}`}
            {page === "resources" && `⚙️ ${getLabel("resources")}`}
            {page === "transport" && `🚚 ${getLabel("transport_lanes")}`}
            {page === "suppliers" && "🏢 Suppliers"}
            {page === "customers" && "👤 Customers"}
            {page === "demands" && `📋 ${getLabel("demand")}`}
            {page === "planned_receipts" && "📥 Planned Receipts"}
            {page === "pricing" && "💰 Pricing"}
            {page === "product_locations" && "📍 Product ↔ Location"}
            {page === "product_resources" && "🔗 Product ↔ Resource"}
            {page === "bom" && "🧬 Bill of Materials"}
            {page === "solver" && "🧮 Run Optimizer"}
            {page === "results" && "📈 Optimization Results"}
            {page === "aliases" && "🏷️ Custom Field Names"}
            {page === "guide" && "📖 Documentation"}
          </h1>
          <div className="header-actions">
            <button className="btn btn-ghost btn-sm" onClick={refreshData}>🔄 Refresh</button>
            <button className="btn btn-ghost btn-sm" onClick={runValidation}>✅ Validate</button>
          </div>
        </header>

        <div className="page-content">
          {page === "dashboard" && <DashboardPage kpis={solverResult?.kpis} locations={locations} products={products} resources={resources} demands={demands} productLocations={productLocations} productResources={productResources} bomEntries={bomEntries} getLabel={getLabel} fmtPct={fmtPct} fmtCost={fmtCost} fmt={fmt} />}
          {page === "locations" && <DataTablePage data={locations} title="Locations"
            columns={["id", "name", "location_type", "country", "region", "active"]} getLabel={getLabel}
            schema={[
              { key: "id", type: "text" }, { key: "name", type: "text" },
              { key: "location_type", type: "select", options: ["Plant", "Warehouse", "DistributionCenter", "Customer", "Supplier"], default: "Plant" },
              { key: "country", type: "text" }, { key: "region", type: "text", default: "" },
            ]}
            onAdd={async (row) => { await invoke("add_location", { location: { ...row, latitude: null, longitude: null, active: true } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_location", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_location", { id: _old.id }); await invoke("add_location", { location: row }); refreshData(); }}
          />}
          {page === "products" && <DataTablePage data={products} title="Products"
            columns={["id", "name", "product_type", "unit_of_measure", "yield_rate", "active"]} getLabel={getLabel}
            schema={[
              { key: "id", type: "text" }, { key: "name", type: "text" },
              { key: "product_type", type: "select", options: ["FinishedGood", "SemiFinished", "RawMaterial", "Packaging"], default: "FinishedGood" },
              { key: "unit_of_measure", type: "text", default: "EA" },
              { key: "yield_rate", type: "number", default: 1.0 },
            ]}
            onAdd={async (row) => { await invoke("add_product", { product: { ...row, weight_kg: 0, volume_m3: 0, shelf_life_days: null, active: true } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_product", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_product", { id: _old.id }); await invoke("add_product", { product: row }); refreshData(); }}
          />}
          {page === "resources" && <DataTablePage data={resources} title="Resources"
            columns={["id", "name", "location_id", "capacity_per_period", "cost_per_unit", "active"]} getLabel={getLabel}
            schema={[
              { key: "id", type: "text" }, { key: "name", type: "text" },
              { key: "location_id", type: "select", options: locations.map(l => l.id) },
              { key: "capacity_per_period", type: "number", default: 10000 },
              { key: "cost_per_unit", type: "number", default: 10 },
            ]}
            onAdd={async (row) => { await invoke("add_resource", { resource: { ...row, capacity_type: "Units", setup_cost: 0, active: true } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_resource", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_resource", { id: _old.id }); await invoke("add_resource", { resource: row }); refreshData(); }}
          />}
          {page === "transport" && <DataTablePage data={transportLanes} title="Transport Lanes"
            columns={["from_location_id", "to_location_id", "mode", "cost_per_unit", "lead_time_days", "lead_time_periods", "tariff_rate"]} getLabel={getLabel}
            schema={[
              { key: "from_location_id", type: "select", options: locations.map(l => l.id) },
              { key: "to_location_id", type: "select", options: locations.map(l => l.id) },
              { key: "mode", type: "select", options: ["Truck", "Ocean", "Air", "Rail", "Intermodal"], default: "Truck" },
              { key: "cost_per_unit", type: "number", default: 1.0 },
              { key: "lead_time_days", type: "number", default: 7 },
              { key: "lead_time_periods", type: "number", default: 1 },
              { key: "tariff_rate", type: "number", default: 0 },
            ]}
            onAdd={async (row) => { await invoke("add_transport_lane", { lane: { ...row, id: crypto.randomUUID(), fixed_cost_per_shipment: 0, min_lot_size: 0, max_lot_size: 1.7976931348623157e+308, co2_per_unit: 0, active: true } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_transport_lane", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_transport_lane", { id: _old.id }); await invoke("add_transport_lane", { lane: row }); refreshData(); }}
          />}
          {page === "suppliers" && <DataTablePage data={suppliers} title="Suppliers"
            columns={["id", "name", "country", "lead_time_days", "capacity_per_period", "quality_rating", "active"]} getLabel={getLabel}
            schema={[
              { key: "id", type: "text" }, { key: "name", type: "text" }, { key: "country", type: "text" },
              { key: "lead_time_days", type: "number", default: 14 },
              { key: "capacity_per_period", type: "number", default: 50000 },
              { key: "quality_rating", type: "number", default: 0.95 },
            ]}
            onAdd={async (row) => { await invoke("add_supplier", { supplier: { ...row, active: true } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_supplier", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_supplier", { id: _old.id }); await invoke("add_supplier", { supplier: row }); refreshData(); }}
          />}
          {page === "customers" && <DataTablePage data={customers} title="Customers"
            columns={["id", "name", "priority", "service_level_target", "country", "active"]} getLabel={getLabel}
            schema={[
              { key: "id", type: "text" }, { key: "name", type: "text" }, { key: "country", type: "text" },
              { key: "priority", type: "number", default: 1 },
              { key: "service_level_target", type: "number", default: 0.95 },
            ]}
            onAdd={async (row) => { await invoke("add_customer", { customer: { ...row, active: true } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_customer", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_customer", { id: _old.id }); await invoke("add_customer", { customer: row }); refreshData(); }}
          />}
          {page === "demands" && <DataTablePage data={demands} title="Demand"
            columns={["product_id", "location_id", "period", "quantity", "demand_type", "customer_id", "is_firm", "priority", "non_delivery_cost"]} getLabel={getLabel}
            schema={[
              { key: "product_id", type: "select", options: products.map(p => p.id) },
              { key: "location_id", type: "select", options: locations.map(l => l.id) },
              { key: "period", type: "number", default: 0 },
              { key: "quantity", type: "number", default: 1000 },
              { key: "demand_type", type: "select", options: ["Forecast", "SalesOrder", "Interplant"], default: "Forecast" },
              { key: "customer_id", type: "select", options: customers.map(c => c.id) },
              { key: "is_firm", type: "bool", default: false },
              { key: "priority", type: "number", default: 1 },
              { key: "non_delivery_cost", type: "number", default: 50 },
            ]}
            onAdd={async (row) => { await invoke("add_demand", { demand: { ...row, id: crypto.randomUUID(), late_delivery_cost: 5, customer_id: row.customer_id || null } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_demand", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_demand", { id: _old.id }); await invoke("add_demand", { demand: row }); refreshData(); }}
          />}
          {page === "planned_receipts" && <DataTablePage data={plannedReceipts} title="Planned Receipts"
            columns={["product_id", "location_id", "period", "quantity", "receipt_type", "source", "is_firm"]} getLabel={getLabel}
            schema={[
              { key: "product_id", type: "select", options: products.map(p => p.id) },
              { key: "location_id", type: "select", options: locations.map(l => l.id) },
              { key: "period", type: "number", default: 0 },
              { key: "quantity", type: "number", default: 1000 },
              { key: "receipt_type", type: "select", options: ["PurchaseOrder", "ProductionOrder", "InTransit"], default: "PurchaseOrder" },
              { key: "source", type: "text", default: "" },
              { key: "is_firm", type: "bool", default: true },
            ]}
            onAdd={async (row) => { await invoke("add_planned_receipt", { receipt: { ...row, id: crypto.randomUUID(), source: row.source || null } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_planned_receipt", { id: row.id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_planned_receipt", { id: _old.id }); await invoke("add_planned_receipt", { receipt: row }); refreshData(); }}
          />}
          {page === "pricing" && <DataTablePage data={productPrices} title="Product Prices"
            columns={["product_id", "location_id", "customer_id", "period", "price_per_unit"]} getLabel={getLabel}
            schema={[
              { key: "product_id", type: "select", options: products.map(p => p.id) },
              { key: "price_per_unit", type: "number", default: 10 },
            ]}
            onAdd={async (row) => { await invoke("add_product_price", { price: { ...row, location_id: null, customer_id: null, period: null } }); refreshData(); }}
          />}
          {page === "product_locations" && <DataTablePage data={productLocations} title="Product-Location"
            columns={["product_id", "location_id", "initial_inventory", "safety_stock", "max_stock", "holding_cost_per_unit", "safety_stock_violation_cost", "max_stock_violation_cost", "can_produce", "can_procure", "can_store"]} getLabel={getLabel}
            schema={[
              { key: "product_id", type: "select", options: products.map(p => p.id) },
              { key: "location_id", type: "select", options: locations.map(l => l.id) },
              { key: "initial_inventory", type: "number", default: 0 },
              { key: "safety_stock", type: "number", default: 0 },
              { key: "max_stock", type: "number", default: 1.7976931348623157e+308 },
              { key: "holding_cost_per_unit", type: "number", default: 0.5 },
              { key: "safety_stock_violation_cost", type: "number", default: 15 },
              { key: "max_stock_violation_cost", type: "number", default: 10 },
              { key: "can_produce", type: "bool", default: false },
              { key: "can_procure", type: "bool", default: false },
            ]}
            onAdd={async (row) => { await invoke("add_product_location", { pl: { ...row, can_store: true, active: true, yield_rate_override: null, production_lead_time: 0, procurement_cost: 0, procurement_lead_time: 0, min_lot_size: 0, max_lot_size: 1.7976931348623157e+308 } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_product_location", { productId: row.product_id, locationId: row.location_id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_product_location", { productId: _old.product_id, locationId: _old.location_id }); await invoke("add_product_location", { pl: row }); refreshData(); }}
          />}
          {page === "product_resources" && <DataTablePage data={productResources} title="Product-Resource"
            columns={["product_id", "resource_id", "location_id", "consumption_rate", "production_rate", "priority"]} getLabel={getLabel}
            schema={[
              { key: "product_id", type: "select", options: products.map(p => p.id) },
              { key: "resource_id", type: "select", options: resources.map(r => r.id) },
              { key: "location_id", type: "select", options: locations.map(l => l.id) },
              { key: "consumption_rate", type: "number", default: 1.0 },
              { key: "production_rate", type: "number", default: 1.0 },
              { key: "priority", type: "number", default: 1 },
            ]}
            onAdd={async (row) => { await invoke("add_product_resource", { pr: { ...row, setup_time: 0, changeover_cost: 0, version_id: "v1", active: true } }); refreshData(); }}
            onDelete={async (row) => { await invoke("delete_product_resource", { productId: row.product_id, resourceId: row.resource_id }); refreshData(); }}
            onUpdate={async (_old, row) => { await invoke("delete_product_resource", { productId: _old.product_id, resourceId: _old.resource_id }); await invoke("add_product_resource", { pr: row }); refreshData(); }}
          />}
          {page === "bom" && <DataTablePage data={bomEntries} title="Bill of Materials"
            columns={["output_product_id", "input_product_id", "quantity_per", "yield_rate"]} getLabel={getLabel}
            schema={[
              { key: "output_product_id", type: "select", options: products.map(p => p.id) },
              { key: "input_product_id", type: "select", options: products.map(p => p.id) },
              { key: "quantity_per", type: "number", default: 1.0 },
              { key: "yield_rate", type: "number", default: 1.0 },
            ]}
            onAdd={async (row) => { await invoke("add_bom_entry", { entry: { ...row, location_id: null } }); refreshData(); }}
          />}
          {page === "solver" && <SolverPage config={solverConfig} setConfig={setSolverConfig} validation={validation} runValidation={runValidation} runSolver={runSolver} solving={solving} />}
          {page === "results" && <ResultsPage result={solverResult} getLabel={getLabel} fmtPct={fmtPct} fmtCost={fmtCost} fmt={fmt} />}
          {page === "aliases" && <AliasPage aliases={aliases} refreshData={refreshData} />}
          {page === "guide" && <GuidePage />}
        </div>

        <div className="status-bar">
          <div><span className="status-dot" />OptiFlow v0.3.0 — HiGHS Solver</div>
          <div>{statusMessage}</div>
          <div>{locations.length} locs · {products.length} prods · {suppliers.length} sups · {customers.length} custs · {demands.length} demands · {plannedReceipts.length} receipts</div>
        </div>
      </div>

      {solving && (
        <div className="solving-overlay">
          <div className="solving-card">
            <div className="spinner" />
            <h3 style={{ marginBottom: 8 }}>Optimizing...</h3>
            <p style={{ color: "var(--text-secondary)", fontSize: 14 }}>HiGHS is solving with inventory balance, BOM explosion, and lead time offsets.</p>
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Dashboard ──────────────────────────────────────────────

function DashboardPage({ kpis, locations, products, resources, demands, productLocations, productResources, bomEntries, getLabel, fmtPct, fmtCost, fmt }: {
  kpis?: DashboardKpis; locations: Location[]; products: Product[]; resources: Resource[];
  demands: Demand[]; productLocations: ProductLocation[]; productResources: ProductResource[];
  bomEntries: BomEntry[]; getLabel: (k: string) => string;
  fmtPct: (n: number) => string; fmtCost: (n: number) => string; fmt: (n: number, d?: number) => string;
}) {
  if (!kpis && demands.length === 0) {
    return (
      <div className="empty-state animate-in">
        <div className="empty-state-icon">🚀</div>
        <div className="empty-state-title">Welcome to OptiFlow</div>
        <div className="empty-state-desc">Load demo data or import your own to get started. The optimizer plans production, transport, and inventory across your supply chain.</div>
      </div>
    );
  }

  return (
    <div className="animate-in">
      <div style={{ marginBottom: 24 }}>
        <h3 style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 12 }}>MODEL SUMMARY</h3>
        <div className="kpi-grid">
          <KpiCard label="Locations" value={String(locations.length)} colorClass="accent" />
          <KpiCard label="Products" value={String(products.length)} colorClass="accent" />
          <KpiCard label="Resources" value={String(resources.length)} colorClass="accent" />
          <KpiCard label="Product-Locations" value={String(productLocations.length)} colorClass="info" />
          <KpiCard label="Product-Resources" value={String(productResources.length)} colorClass="info" />
          <KpiCard label="BOM Entries" value={String(bomEntries.length)} colorClass="info" />
          <KpiCard label="Demand Records" value={String(demands.length)} colorClass="accent" />
          <KpiCard label="Total Demand" value={fmt(demands.reduce((s, d) => s + d.quantity, 0))} colorClass="info" />
          <KpiCard label="Total Init. Inventory" value={fmt(productLocations.reduce((s, pl) => s + pl.initial_inventory, 0))} colorClass="success" />
        </div>
      </div>

      {kpis && (
        <div>
          <h3 style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 12 }}>OPTIMIZATION RESULTS</h3>
          <div className="kpi-grid">
            <KpiCard label={getLabel("demand_fulfillment_pct")} value={fmtPct(kpis.demand_fulfillment_pct)} colorClass={kpis.demand_fulfillment_pct >= 95 ? "success" : kpis.demand_fulfillment_pct >= 80 ? "warning" : "danger"} />
            <KpiCard label={getLabel("total_cost")} value={fmtCost(kpis.total_cost)} colorClass="accent" />
            <KpiCard label={getLabel("cost_per_unit_delivered")} value={fmtCost(kpis.cost_per_unit_delivered)} colorClass="info" />
            <KpiCard label={getLabel("total_delivered")} value={fmt(kpis.total_delivered)} colorClass="success" />
            <KpiCard label={getLabel("total_unmet")} value={fmt(kpis.total_unmet)} colorClass={kpis.total_unmet > 0 ? "danger" : "success"} />
            <KpiCard label={getLabel("production_cost")} value={fmtCost(kpis.production_cost)} colorClass="accent" />
            <KpiCard label={getLabel("transport_cost")} value={fmtCost(kpis.transport_cost)} colorClass="accent" />
            <KpiCard label={getLabel("holding_cost")} value={fmtCost(kpis.holding_cost)} colorClass="accent" />
            <KpiCard label={getLabel("penalty_cost")} value={fmtCost(kpis.penalty_cost)} colorClass={kpis.penalty_cost > 0 ? "warning" : "success"} />
            <KpiCard label={getLabel("avg_capacity_utilization")} value={fmtPct(kpis.avg_capacity_utilization)} colorClass={kpis.avg_capacity_utilization > 90 ? "danger" : "info"} />
            <KpiCard label={getLabel("avg_inventory")} value={fmt(kpis.avg_inventory)} colorClass="info" />
            <KpiCard label={getLabel("num_bottleneck_resources")} value={String(kpis.num_bottleneck_resources)} colorClass={kpis.num_bottleneck_resources > 0 ? "danger" : "success"} />
          </div>
        </div>
      )}
    </div>
  );
}

function KpiCard({ label, value, colorClass }: { label: string; value: string; colorClass: string }) {
  return <div className="kpi-card"><div className="kpi-label">{label}</div><div className={`kpi-value ${colorClass}`}>{value}</div></div>;
}

// ─── Column Schema (drives forms + tables generically) ──────

interface ColSchema { key: string; type: "text" | "number" | "bool" | "select"; default?: any; options?: string[]; required?: boolean; }

function buildDefaults(schema: ColSchema[]): Record<string, any> {
  const obj: Record<string, any> = {};
  for (const col of schema) obj[col.key] = col.default ?? (col.type === "number" ? 0 : col.type === "bool" ? false : "");
  return obj;
}

// ─── Interactive Data Table (with inline edit, CSV export/import) ──

function DataTablePage({ data, columns, schema, getLabel, title, onAdd, onDelete, onUpdate }: {
  data: any[]; columns: string[]; schema?: ColSchema[]; getLabel: (k: string) => string; title: string;
  onAdd?: (row: any) => void; onDelete?: (row: any) => void; onUpdate?: (oldRow: any, newRow: any) => void;
}) {
  const [showForm, setShowForm] = useState(false);
  const [formData, setFormData] = useState<Record<string, any>>(() => schema ? buildDefaults(schema) : {});
  const [editCell, setEditCell] = useState<{ row: number; col: string } | null>(null);
  const [editValue, setEditValue] = useState<string>("");
  const editRef = useRef<HTMLInputElement | HTMLSelectElement>(null);

  const handleAdd = () => {
    if (onAdd) { onAdd(formData); setFormData(schema ? buildDefaults(schema) : {}); setShowForm(false); }
  };

  // ─── Inline Edit ───
  const startEdit = (rowIdx: number, colKey: string, currentValue: any) => {
    if (!onUpdate) return;
    setEditCell({ row: rowIdx, col: colKey });
    setEditValue(currentValue === null || currentValue === undefined ? "" :
      currentValue === 1.7976931348623157e+308 ? "" : String(currentValue));
  };

  const commitEdit = (rowIdx: number, colKey: string) => {
    if (!onUpdate || !editCell) return;
    const row = data[rowIdx];
    const colSchema = schema?.find(s => s.key === colKey);
    let newVal: any = editValue;
    if (colSchema?.type === "number") newVal = editValue === "" ? 0 : Number(editValue);
    else if (colSchema?.type === "bool") newVal = editValue === "true";
    if (row[colKey] !== newVal) {
      const updated = { ...row, [colKey]: newVal };
      onUpdate(row, updated);
    }
    setEditCell(null);
  };

  const cancelEdit = () => setEditCell(null);

  useEffect(() => { if (editRef.current) editRef.current.focus(); }, [editCell]);

  // ─── CSV Export ───
  const exportCSV = () => {
    if (data.length === 0) return;
    const header = columns.join(",");
    const rows = data.map(row =>
      columns.map(col => {
        const v = row[col];
        if (v === null || v === undefined) return "";
        if (v === 1.7976931348623157e+308) return "MAX";
        if (typeof v === "string" && (v.includes(",") || v.includes('"'))) return `"${v.replace(/"/g, '""')}"`;
        return String(v);
      }).join(",")
    );
    const csv = [header, ...rows].join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = `${title.toLowerCase().replace(/\s+/g, "_")}.csv`; a.click();
    URL.revokeObjectURL(url);
  };

  // ─── CSV Import ───
  const importCSV = (file: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const text = e.target?.result as string;
      if (!text || !onAdd) return;
      const lines = text.split(/\r?\n/).filter(l => l.trim());
      if (lines.length < 2) return;
      const headers = lines[0].split(",").map(h => h.trim().replace(/^"|"$/g, ""));
      let imported = 0;
      for (let i = 1; i < lines.length; i++) {
        const vals = parseCSVLine(lines[i]);
        if (vals.length !== headers.length) continue;
        const row: Record<string, any> = {};
        headers.forEach((h, idx) => {
          const colSchema = schema?.find(s => s.key === h);
          let v: any = vals[idx];
          if (v === "MAX" || v === "∞") v = 1.7976931348623157e+308;
          else if (colSchema?.type === "number") v = v === "" ? 0 : Number(v);
          else if (colSchema?.type === "bool") v = v === "true" || v === "✅" || v === "1";
          row[h] = v;
        });
        onAdd(row);
        imported++;
      }
      alert(`Imported ${imported} records into ${title}`);
    };
    reader.readAsText(file);
  };

  const handleFileSelect = () => {
    const input = document.createElement("input");
    input.type = "file"; input.accept = ".csv";
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) importCSV(file);
    };
    input.click();
  };

  // ─── Render ───
  const getColSchema = (colKey: string) => schema?.find(s => s.key === colKey);

  return (
    <div className="animate-in">
      <div style={{ marginBottom: 16, display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 8 }}>
        <span style={{ color: "var(--text-secondary)", fontSize: 13 }}>{data.length} records</span>
        <div style={{ display: "flex", gap: 6 }}>
          {data.length > 0 && (
            <button className="btn btn-ghost btn-sm" onClick={exportCSV} title="Download CSV">📥 Export CSV</button>
          )}
          {onAdd && schema && (
            <button className="btn btn-ghost btn-sm" onClick={handleFileSelect} title="Upload CSV">📤 Import CSV</button>
          )}
          {onAdd && schema && (
            <button className="btn btn-primary btn-sm" onClick={() => setShowForm(!showForm)}>
              {showForm ? "✕ Cancel" : "➕ Add Record"}
            </button>
          )}
        </div>
      </div>

      {showForm && schema && (
        <div className="card" style={{ marginBottom: 16 }}>
          <div className="card-header"><div className="card-title">Add {title}</div></div>
          <div className="card-body" style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 10 }}>
            {schema.map((col) => (
              <div className="form-group" key={col.key} style={{ margin: 0 }}>
                <label className="form-label" style={{ fontSize: 11 }}>{getLabel(col.key)}</label>
                {col.type === "select" ? (
                  <select className="form-input form-select" value={formData[col.key] || ""}
                    onChange={(e) => setFormData({ ...formData, [col.key]: e.target.value })}>
                    <option value="">—</option>
                    {col.options?.map((o) => <option key={o} value={o}>{o}</option>)}
                  </select>
                ) : col.type === "bool" ? (
                  <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13 }}>
                    <input type="checkbox" checked={!!formData[col.key]}
                      onChange={(e) => setFormData({ ...formData, [col.key]: e.target.checked })} /> Yes
                  </label>
                ) : (
                  <input className="form-input" type={col.type === "number" ? "number" : "text"}
                    step={col.type === "number" ? "any" : undefined}
                    value={formData[col.key] ?? ""} placeholder={getLabel(col.key)}
                    onChange={(e) => setFormData({ ...formData, [col.key]: col.type === "number" ? Number(e.target.value) : e.target.value })} />
                )}
              </div>
            ))}
            <div style={{ display: "flex", alignItems: "end" }}>
              <button className="btn btn-success btn-sm" onClick={handleAdd}>✅ Save</button>
            </div>
          </div>
        </div>
      )}

      {data.length === 0 && !showForm ? (
        <div className="empty-state"><div className="empty-state-icon">📭</div><div className="empty-state-title">No {title} Yet</div><div className="empty-state-desc">Click "➕ Add Record", import a CSV, or load demo data.</div></div>
      ) : data.length > 0 && (
        <div className="data-table-wrapper">
          <table className="data-table">
            <thead><tr>{columns.map((col) => <th key={col}>{getLabel(col)}</th>)}{onDelete && <th style={{ width: 40 }}>Del</th>}</tr></thead>
            <tbody>{data.map((row, ri) => (
              <tr key={ri}>
                {columns.map((col) => {
                  const isEditing = editCell?.row === ri && editCell?.col === col;
                  const cs = getColSchema(col);
                  if (isEditing && cs) {
                    return <td key={col} className="editing-cell">{cs.type === "select" ? (
                      <select ref={editRef as any} className="inline-edit-input" value={editValue}
                        onChange={(e) => setEditValue(e.target.value)}
                        onBlur={() => commitEdit(ri, col)}
                        onKeyDown={(e) => { if (e.key === "Enter") commitEdit(ri, col); if (e.key === "Escape") cancelEdit(); }}>
                        <option value="">—</option>
                        {cs.options?.map(o => <option key={o} value={o}>{o}</option>)}
                      </select>
                    ) : cs.type === "bool" ? (
                      <select ref={editRef as any} className="inline-edit-input" value={editValue}
                        onChange={(e) => { setEditValue(e.target.value); }}
                        onBlur={() => commitEdit(ri, col)}
                        onKeyDown={(e) => { if (e.key === "Enter") commitEdit(ri, col); if (e.key === "Escape") cancelEdit(); }}>
                        <option value="true">✅ Yes</option>
                        <option value="false">❌ No</option>
                      </select>
                    ) : (
                      <input ref={editRef as any} className="inline-edit-input"
                        type={cs.type === "number" ? "number" : "text"} step="any"
                        value={editValue} onChange={(e) => setEditValue(e.target.value)}
                        onBlur={() => commitEdit(ri, col)}
                        onKeyDown={(e) => { if (e.key === "Enter") commitEdit(ri, col); if (e.key === "Escape") cancelEdit(); }} />
                    )}</td>;
                  }
                  return <td key={col}
                    className={onUpdate && cs ? "editable-cell" : ""}
                    onClick={() => cs && startEdit(ri, col, row[col])}
                    title={onUpdate && cs ? "Click to edit" : undefined}
                  >{formatCell(row[col])}</td>;
                })}
                {onDelete && <td><button className="btn btn-ghost btn-sm" style={{ color: "var(--danger)", padding: "2px 6px" }}
                  onClick={() => onDelete(row)}>🗑️</button></td>}
              </tr>
            ))}</tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function parseCSVLine(line: string): string[] {
  const result: string[] = [];
  let current = "";
  let inQuotes = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (inQuotes) {
      if (ch === '"' && line[i + 1] === '"') { current += '"'; i++; }
      else if (ch === '"') inQuotes = false;
      else current += ch;
    } else {
      if (ch === '"') inQuotes = true;
      else if (ch === ',') { result.push(current.trim()); current = ""; }
      else current += ch;
    }
  }
  result.push(current.trim());
  return result;
}

function formatCell(value: unknown): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "boolean") return value ? "✅" : "❌";
  if (typeof value === "number") {
    if (value === 1.7976931348623157e+308) return "∞"; // f64::MAX
    if (value >= 1000) return value.toLocaleString();
    if (Number.isInteger(value)) return String(value);
    return value.toFixed(2);
  }
  return String(value);
}

// ─── Solver Page ────────────────────────────────────────────

function SolverPage({ config, setConfig, validation, runValidation, runSolver, solving }: {
  config: SolverConfig; setConfig: (c: SolverConfig) => void;
  validation: ValidationMessage[]; runValidation: () => void; runSolver: () => void; solving: boolean;
}) {
  return (
    <div className="animate-in">
      <div className="solve-panel">
        <div className="card">
          <div className="card-header"><div className="card-title">⚙️ Solver Configuration</div></div>
          <div className="card-body solve-config">
            <div className="form-group">
              <label className="form-label">Objective</label>
              <select className="form-input form-select" value={config.objective}
                onChange={(e) => setConfig({ ...config, objective: e.target.value })}>
                <option value="MinimizeCost">Minimize Total Cost</option>
                <option value="MaximizeDelivery">Maximize Delivery</option>
                <option value="MaximizeProfit">Maximize Profit</option>
              </select>
            </div>
            <div className="form-group">
              <label className="form-label">Time Limit (seconds)</label>
              <input type="number" className="form-input" value={config.time_limit_seconds}
                onChange={(e) => setConfig({ ...config, time_limit_seconds: Number(e.target.value) })} />
            </div>
            <div className="form-group">
              <label className="form-label">Optimality Gap</label>
              <input type="number" className="form-input" step="0.001" value={config.optimality_gap}
                onChange={(e) => setConfig({ ...config, optimality_gap: Number(e.target.value) })} />
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-secondary btn-lg" onClick={runValidation}>✅ Validate</button>
              <button className="btn btn-primary btn-lg" onClick={runSolver} disabled={solving} style={{ flex: 1 }}>
                {solving ? "Solving..." : "🚀 Run Optimizer"}
              </button>
            </div>
          </div>
        </div>

        <div className="card">
          <div className="card-header"><div className="card-title">🔒 Time Fences</div></div>
          <div className="card-body solve-config">
            <p style={{ color: "var(--text-muted)", fontSize: 12, marginBottom: 12 }}>
              Frozen: no changes allowed. Firm: changes penalized. Free: full optimization.
            </p>
            <div className="form-group">
              <label className="form-label">Frozen Periods (locked, no production/transport)</label>
              <input type="number" className="form-input" min={0} value={config.frozen_periods}
                onChange={(e) => setConfig({ ...config, frozen_periods: Number(e.target.value) })} />
            </div>
            <div className="form-group">
              <label className="form-label">Firm Periods (change penalty applies)</label>
              <input type="number" className="form-input" min={0} value={config.firm_periods}
                onChange={(e) => setConfig({ ...config, firm_periods: Number(e.target.value) })} />
            </div>
            <div className="form-group">
              <label className="form-label">Firm Zone Change Penalty ($)</label>
              <input type="number" className="form-input" min={0} step={10} value={config.firm_change_penalty}
                onChange={(e) => setConfig({ ...config, firm_change_penalty: Number(e.target.value) })} />
            </div>
            {config.frozen_periods > 0 && (
              <div style={{ background: "var(--bg-secondary)", borderRadius: 6, padding: "8px 12px", fontSize: 12, color: "var(--text-secondary)" }}>
                ⚠️ Periods 0–{config.frozen_periods - 1} are <strong>frozen</strong> — only initial inventory and planned receipts will serve demand.
                {config.firm_periods > 0 && <> Periods {config.frozen_periods}–{config.frozen_periods + config.firm_periods - 1} are <strong>firm</strong> (${config.firm_change_penalty} change penalty).</>}
              </div>
            )}
          </div>
        </div>

        <div className="card">
          <div className="card-header"><div className="card-title">🔍 Validation</div><span className="badge badge-info">{validation.length} checks</span></div>
          <div className="card-body">
            {validation.length === 0 ? (
              <p style={{ color: "var(--text-muted)", fontSize: 13 }}>Click "Validate" to check your model.</p>
            ) : (
              <div className="validation-list">
                {validation.map((v, i) => (
                  <div key={i} className={`validation-item ${v.severity.toLowerCase()}`}>
                    <span className="validation-icon">{v.severity === "Error" ? "❌" : v.severity === "Warning" ? "⚠️" : "ℹ️"}</span>
                    <div>
                      <div className="validation-message"><strong>[{v.category}]</strong> {v.message}</div>
                      {v.suggestion && <div className="validation-suggestion">💡 {v.suggestion}</div>}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── Results Page ───────────────────────────────────────────

function ResultsPage({ result, getLabel, fmtPct, fmtCost, fmt }: {
  result: SolverResult | null; getLabel: (k: string) => string;
  fmtPct: (n: number) => string; fmtCost: (n: number) => string; fmt: (n: number, d?: number) => string;
}) {
  const [tab, setTab] = useState<"kpis" | "production" | "transport" | "inventory" | "unmet" | "capacity">("kpis");
  const [showExplain, setShowExplain] = useState(false);

  if (!result) {
    return <div className="empty-state animate-in"><div className="empty-state-icon">🧮</div><div className="empty-state-title">No Results Yet</div><div className="empty-state-desc">Run the optimizer to see results.</div></div>;
  }

  const statusStr = typeof result.status === "string" ? result.status : `Error: ${(result.status as { Error: string }).Error}`;

  return (
    <div className="animate-in" style={{ display: "flex", height: "100%" }}>
      <div style={{ flex: 1, minWidth: 0, overflowY: "auto" }}>
        <div style={{ display: "flex", gap: 16, marginBottom: 20, alignItems: "center" }}>
          <span className={`badge ${statusStr === "Optimal" ? "badge-success" : statusStr === "Feasible" ? "badge-warning" : "badge-danger"}`}>{statusStr}</span>
          <span style={{ color: "var(--text-secondary)", fontSize: 13 }}>Solved in {result.solve_time_ms}ms · Objective: {fmtCost(result.objective_value)}</span>
        </div>

        <div className="tabs">
          {(["kpis", "production", "transport", "inventory", "unmet", "capacity"] as const).map((t) => (
            <button key={t} className={`tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
              {t === "kpis" ? "📊 KPIs" : t === "production" ? "🏭 Production" : t === "transport" ? "🚚 Transport" : t === "inventory" ? "📦 Inventory" : t === "unmet" ? "⚠️ Unmet" : "📏 Capacity"}
            </button>
          ))}
        </div>

        {tab === "kpis" && (
          <div className="kpi-grid">
            <KpiCard label={getLabel("demand_fulfillment_pct")} value={fmtPct(result.kpis.demand_fulfillment_pct)} colorClass={result.kpis.demand_fulfillment_pct >= 95 ? "success" : "warning"} />
            <KpiCard label={getLabel("total_cost")} value={fmtCost(result.kpis.total_cost)} colorClass="accent" />
            <KpiCard label={getLabel("cost_per_unit_delivered")} value={fmtCost(result.kpis.cost_per_unit_delivered)} colorClass="info" />
            <KpiCard label={getLabel("total_delivered")} value={fmt(result.kpis.total_delivered)} colorClass="success" />
            <KpiCard label={getLabel("total_unmet")} value={fmt(result.kpis.total_unmet)} colorClass={result.kpis.total_unmet > 0 ? "danger" : "success"} />
            <KpiCard label={getLabel("production_cost")} value={fmtCost(result.kpis.production_cost)} colorClass="accent" />
            <KpiCard label={getLabel("transport_cost")} value={fmtCost(result.kpis.transport_cost)} colorClass="accent" />
            <KpiCard label={getLabel("holding_cost")} value={fmtCost(result.kpis.holding_cost)} colorClass="accent" />
            <KpiCard label={getLabel("penalty_cost")} value={fmtCost(result.kpis.penalty_cost)} colorClass={result.kpis.penalty_cost > 0 ? "warning" : "success"} />
            <KpiCard label={getLabel("avg_capacity_utilization")} value={fmtPct(result.kpis.avg_capacity_utilization)} colorClass="info" />
            <KpiCard label={getLabel("avg_inventory")} value={fmt(result.kpis.avg_inventory)} colorClass="info" />
            <KpiCard label="Peak Inventory" value={fmt(result.kpis.peak_inventory)} colorClass="info" />
          </div>
        )}

        {tab === "production" && (
          <div className="data-table-wrapper"><table className="data-table">
            <thead><tr><th>Product</th><th>Location</th><th>Resource</th><th>Period</th><th>Quantity</th><th>Cost</th></tr></thead>
            <tbody>{result.production_plan.map((p, i) => (
              <tr key={i}><td>{p.product_id}</td><td>{p.location_id}</td><td>{p.resource_id}</td><td>P{p.period + 1}</td><td>{fmt(p.quantity)}</td><td>{fmtCost(p.cost)}</td></tr>
            ))}</tbody>
          </table></div>
        )}

        {tab === "transport" && (
          <div className="data-table-wrapper"><table className="data-table">
            <thead><tr><th>Product</th><th>From</th><th>To</th><th>Mode</th><th>Period</th><th>Quantity</th><th>Cost</th></tr></thead>
            <tbody>{result.transport_plan.map((t, i) => (
              <tr key={i}><td>{t.product_id}</td><td>{t.from_location_id}</td><td>{t.to_location_id}</td><td>{String(t.mode)}</td><td>P{t.period + 1}</td><td>{fmt(t.quantity)}</td><td>{fmtCost(t.cost)}</td></tr>
            ))}</tbody>
          </table></div>
        )}

        {tab === "inventory" && (
          result.inventory_plan.length === 0 ? (
            <div className="empty-state"><div className="empty-state-icon">📦</div><div className="empty-state-title">No Inventory Data</div><div className="empty-state-desc">Add Product-Location records to enable inventory tracking.</div></div>
          ) : (
            <div className="data-table-wrapper"><table className="data-table">
              <thead><tr><th>Product</th><th>Location</th><th>Period</th><th>Stock Level</th><th>Holding Cost</th><th>SS Delta</th></tr></thead>
              <tbody>{result.inventory_plan.map((inv, i) => (
                <tr key={i}>
                  <td>{inv.product_id}</td><td>{inv.location_id}</td><td>P{inv.period + 1}</td>
                  <td>{fmt(inv.quantity)}</td><td>{fmtCost(inv.holding_cost)}</td>
                  <td style={{ color: inv.safety_stock_delta < 0 ? "var(--danger)" : "var(--success)" }}>
                    {inv.safety_stock_delta >= 0 ? "+" : ""}{fmt(inv.safety_stock_delta)}
                  </td>
                </tr>
              ))}</tbody>
            </table></div>
          )
        )}

        {tab === "unmet" && (
          result.unmet_demand.length === 0 ? (
            <div className="empty-state"><div className="empty-state-icon">✅</div><div className="empty-state-title">All Demand Met!</div></div>
          ) : (
            <div className="data-table-wrapper"><table className="data-table">
              <thead><tr><th>Product</th><th>Location</th><th>Period</th><th>Unmet Qty</th><th>Penalty</th><th>Reason</th></tr></thead>
              <tbody>{result.unmet_demand.map((u, i) => (
                <tr key={i}><td>{u.product_id}</td><td>{u.location_id}</td><td>P{u.period + 1}</td>
                  <td style={{ color: "var(--danger)" }}>{fmt(u.unmet_quantity)}</td><td>{fmtCost(u.penalty_cost)}</td><td>{u.reason}</td></tr>
              ))}</tbody>
            </table></div>
          )
        )}

        {tab === "capacity" && (
          <div className="data-table-wrapper"><table className="data-table">
            <thead><tr><th>Resource</th><th>Location</th><th>Period</th><th>Used</th><th>Available</th><th>Utilization</th></tr></thead>
            <tbody>{result.capacity_utilization.map((c, i) => (
              <tr key={i}>
                <td>{c.resource_id}</td><td>{c.location_id}</td><td>P{c.period + 1}</td><td>{fmt(c.used)}</td><td>{fmt(c.available)}</td>
                <td><span className={`badge ${c.utilization_pct > 95 ? "badge-danger" : c.utilization_pct > 80 ? "badge-warning" : "badge-success"}`}>{fmtPct(c.utilization_pct)}</span></td>
              </tr>
            ))}</tbody>
          </table></div>
        )}
      </div>

      {showExplain && <ExplainPanel onClose={() => setShowExplain(false)} />}

      <button
        className={`explain-toggle ${showExplain ? "open" : ""}`}
        onClick={() => setShowExplain(!showExplain)}
        title="Ask about optimizer decisions"
      >
        {showExplain ? "✕" : "💬"}
      </button>
    </div>
  );
}

// ─── Explain Panel (Chat) ────────────────────────────────────

interface ChatMessage {
  role: "user" | "system";
  text: string;
  dataPoints?: ExplainDataPoint[];
  suggestions?: string[];
}

function ExplainPanel({ onClose }: { onClose: () => void }) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => { scrollToBottom(); }, [messages]);

  const askQuestion = async (question: string) => {
    if (!question.trim()) return;
    const userMsg: ChatMessage = { role: "user", text: question };
    setMessages((prev) => [...prev, userMsg]);
    setInput("");
    setLoading(true);

    try {
      const resp = await invoke<ExplainResponse>("explain_decision", { question });
      const sysMsg: ChatMessage = {
        role: "system",
        text: resp.answer,
        dataPoints: resp.data_points,
        suggestions: resp.suggestions,
      };
      setMessages((prev) => [...prev, sysMsg]);
    } catch (e) {
      setMessages((prev) => [...prev, { role: "system", text: `Error: ${e}` }]);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      askQuestion(input);
    }
  };

  const starterQuestions = [
    "Summarize the results",
    "Why is there unmet demand?",
    "What are the bottleneck resources?",
    "What is driving the cost?",
    "Why is inventory building up?",
  ];

  // Render **bold** markdown
  const renderText = (text: string) => {
    const parts = text.split(/(\*\*.*?\*\*)/);
    return parts.map((part, i) =>
      part.startsWith("**") && part.endsWith("**")
        ? <strong key={i}>{part.slice(2, -2)}</strong>
        : part
    );
  };

  return (
    <div className="explain-panel">
      <div className="explain-header">
        <div className="explain-header-title">
          🧠 <span>OptiFlow Explain</span>
        </div>
        <button className="explain-close" onClick={onClose}>✕</button>
      </div>

      <div className="explain-messages">
        {messages.length === 0 && (
          <div className="explain-welcome">
            <div className="explain-welcome-icon">🧠</div>
            <div className="explain-welcome-title">Ask about the results</div>
            <div className="explain-welcome-desc">
              I can explain why the optimizer made specific decisions by analyzing the master data and solver output.
            </div>
            <div className="explain-suggestions">
              {starterQuestions.map((q, i) => (
                <button key={i} className="explain-suggestion-chip" onClick={() => askQuestion(q)}>{q}</button>
              ))}
            </div>
          </div>
        )}

        {messages.map((msg, i) => (
          <div key={i} className={`explain-bubble ${msg.role}`}>
            {msg.role === "user" ? msg.text : renderText(msg.text)}
            {msg.dataPoints && msg.dataPoints.length > 0 && (
              <div className="explain-data-cards">
                {msg.dataPoints.slice(0, 6).map((dp, j) => (
                  <div key={j} className="explain-data-card">
                    <div className="explain-data-card-label">{dp.label}</div>
                    <div className="explain-data-card-value">{dp.value}</div>
                    {dp.context && <div className="explain-data-card-context">{dp.context}</div>}
                  </div>
                ))}
              </div>
            )}
            {msg.suggestions && msg.suggestions.length > 0 && i === messages.length - 1 && (
              <div className="explain-suggestions">
                {msg.suggestions.map((s, j) => (
                  <button key={j} className="explain-suggestion-chip" onClick={() => askQuestion(s)}>{s}</button>
                ))}
              </div>
            )}
          </div>
        ))}

        {loading && (
          <div className="explain-loading">
            <div className="explain-loading-dots">
              <div className="explain-loading-dot" />
              <div className="explain-loading-dot" />
              <div className="explain-loading-dot" />
            </div>
            Analyzing...
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className="explain-input-area">
        <input
          className="explain-input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Ask about optimizer decisions..."
          disabled={loading}
        />
        <button className="explain-send" onClick={() => askQuestion(input)} disabled={loading || !input.trim()}>→</button>
      </div>
    </div>
  );
}

// ─── Alias Editor ───────────────────────────────────────────

function AliasPage({ aliases, refreshData }: { aliases: AliasMap; refreshData: () => void }) {
  const [editing, setEditing] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  const saveAlias = async (key: string) => {
    try { await invoke("set_alias", { internalName: key, displayName: editValue }); setEditing(null); refreshData(); }
    catch (e) { console.error("Failed to save:", e); }
  };

  return (
    <div className="animate-in">
      <p style={{ color: "var(--text-secondary)", fontSize: 14, marginBottom: 20 }}>Customize field labels to match your business terminology.</p>
      <div className="data-table-wrapper"><table className="data-table">
        <thead><tr><th>Internal Field</th><th>Display Name</th><th>Action</th></tr></thead>
        <tbody>{Object.entries(aliases.aliases).sort().map(([key, display]) => (
          <tr key={key}>
            <td style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>{key}</td>
            <td>{editing === key ? <input className="form-input" value={editValue} onChange={(e) => setEditValue(e.target.value)} onKeyDown={(e) => e.key === "Enter" && saveAlias(key)} autoFocus style={{ maxWidth: 200 }} /> : display}</td>
            <td>{editing === key ? (
              <div style={{ display: "flex", gap: 4 }}>
                <button className="btn btn-success btn-sm" onClick={() => saveAlias(key)}>Save</button>
                <button className="btn btn-ghost btn-sm" onClick={() => setEditing(null)}>Cancel</button>
              </div>
            ) : <button className="btn btn-ghost btn-sm" onClick={() => { setEditing(key); setEditValue(display); }}>✏️ Edit</button>}</td>
          </tr>
        ))}</tbody>
      </table></div>
    </div>
  );
}

// ─── Guide Page ─────────────────────────────────────────────

function GuidePage() {
  return (
    <div className="animate-in" style={{ maxWidth: 800 }}>
      <div className="card" style={{ marginBottom: 20 }}>
        <div className="card-header"><div className="card-title">🚀 Getting Started</div></div>
        <div className="card-body" style={{ lineHeight: 1.8, color: "var(--text-secondary)" }}>
          <ol style={{ paddingLeft: 20 }}>
            <li><strong>Load Demo Data</strong> — Populates a semiconductor scenario with 5 locations, 3 products, BOM, and inventory.</li>
            <li><strong>Review Master Data</strong> — Check Locations, Products, Resources, and the new relationship tables.</li>
            <li><strong>Check Relationships</strong> — Product↔Location controls inventory. Product↔Resource controls capacity consumption.</li>
            <li><strong>Validate</strong> — Go to "Run Optimizer" and click Validate.</li>
            <li><strong>Solve</strong> — Set objective and click "Run Optimizer".</li>
            <li><strong>Analyze</strong> — Review KPIs, production plan, transport, inventory levels, and capacity.</li>
          </ol>
        </div>
      </div>

      <div className="card" style={{ marginBottom: 20 }}>
        <div className="card-header"><div className="card-title">📍 Relationship Tables (NEW)</div></div>
        <div className="card-body" style={{ lineHeight: 1.8, color: "var(--text-secondary)" }}>
          <p><strong>Product ↔ Location</strong> — Defines WHERE each product exists. Controls initial inventory, safety stock, holding costs, and whether a product can be produced, procured, or stored at that location.</p>
          <p><strong>Product ↔ Resource</strong> — Defines HOW MUCH resource capacity each product consumes. A chip might take 1.2 resource units, while a simpler part takes 0.5.</p>
          <p><strong>Bill of Materials</strong> — Defines WHAT INPUTS are needed. Making 1 chip requires 0.5 wafers. The solver automatically consumes components.</p>
        </div>
      </div>

      <div className="card" style={{ marginBottom: 20 }}>
        <div className="card-header"><div className="card-title">🧮 Solver Capabilities</div></div>
        <div className="card-body" style={{ lineHeight: 1.8, color: "var(--text-secondary)" }}>
          <p><strong>Inventory Balance</strong> — Stock[t] = Stock[t-1] + Production + Inbound − Outbound − Demand. True multi-period planning.</p>
          <p><strong>Lead Time Offsets</strong> — Product shipped in Period 1 arrives in Period 2 (if lead time = 1 period).</p>
          <p><strong>BOM Explosion</strong> — Producing finished goods automatically consumes raw materials.</p>
          <p><strong>Safety Stock</strong> — Soft constraint with penalty cost. Optimizer pre-builds inventory to maintain safety stock.</p>
          <p><strong>Calendar-Based Capacity</strong> — Resources can have different capacity per period (holidays, shutdowns).</p>
        </div>
      </div>

      <div className="card">
        <div className="card-header"><div className="card-title">💰 Cost Tuning Guide</div></div>
        <div className="card-body" style={{ lineHeight: 1.8, color: "var(--text-secondary)" }}>
          <p><strong>Non-Delivery Cost</strong> — Set HIGHER than production + transport to ensure delivery. Lower = optimizer may skip.</p>
          <p><strong>Holding Cost</strong> — Higher = less inventory. Lower = more pre-build.</p>
          <p><strong>Safety Stock Violation Cost</strong> — Higher = more aggressively maintains safety stock buffer.</p>
          <p><strong>Tariff Rate</strong> — Added to transport cost. 25% tariff on a $1 shipment = $1.25 total.</p>
        </div>
      </div>
    </div>
  );
}

export default App;
