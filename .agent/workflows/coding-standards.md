---
description: Production code standards for OptiFlow — MUST follow for all code changes
---

# OptiFlow Coding Standards

// turbo-all

These rules are MANDATORY for all code written in this project. No exceptions.

## 1. File Structure — Modular Architecture

### Rust Backend (`src-tauri/src/`)

Split code by DOMAIN, not by layer. Each domain module contains its own types, logic, and commands.

```
src-tauri/src/
├── main.rs                     # Entry point only
├── lib.rs                      # Module declarations + Tauri setup
├── state.rs                    # AppState struct (single source of truth)
├── models/
│   ├── mod.rs                  # Re-exports all types
│   ├── master.rs               # Location, Product, Resource, TransportLane
│   ├── relationships.rs        # ProductLocation, ProductResource, BomEntry, SourcingRule
│   ├── transactions.rs         # Demand, PlannedReceipt, ProductPrice
│   ├── calendar.rs             # PlanningCalendar, PlanningBucket, CalendarEntry, FenceZone
│   ├── solver_types.rs         # SolverConfig, SolverResult, PlanEntry, KPIs
│   └── enums.rs                # All enums (LocationType, ProductType, etc.)
├── commands/
│   ├── mod.rs                  # Re-exports all command functions
│   ├── crud.rs                 # Generic CRUD macro/functions
│   ├── master_commands.rs      # Location, Product, Resource, Transport commands
│   ├── relationship_commands.rs# PL, PR, BOM commands
│   ├── transaction_commands.rs # Demand, PlannedReceipt, Price commands
│   ├── solver_commands.rs      # run_optimizer, validate_model
│   └── io_commands.rs          # save, load, import_csv, export
├── solver/
│   ├── mod.rs                  # Public API: solve()
│   ├── variables.rs            # Variable creation (production, transport, inventory, etc.)
│   ├── constraints.rs          # Constraint building (balance, capacity, time fences)
│   ├── results.rs              # Result extraction (plans, KPIs, receipts, net reqs)
│   └── helpers.rs              # Cost lookups, lead time, yield rate
├── validator.rs                # Pre-solve validation
└── persistence.rs              # JSON save/load, CSV import/export
```

### React Frontend (`src/`)

```
src/
├── App.tsx                     # Router + layout only (< 100 lines)
├── index.css                   # Design system (keep as-is)
├── types.ts                    # ALL TypeScript interfaces (single source of truth)
├── hooks/
│   ├── useInvoke.ts            # Safe invoke wrapper with error handling
│   └── useModel.ts             # All data fetching + state management
├── components/
│   ├── Layout.tsx              # Sidebar + header + status bar
│   ├── DataTable.tsx           # Generic data table (reusable for ALL entity views)
│   ├── KpiCard.tsx             # Single KPI card
│   ├── KpiGrid.tsx             # Grid of KPIs
│   └── EmptyState.tsx          # Empty state component
├── pages/
│   ├── Dashboard.tsx
│   ├── MasterDataPage.tsx      # Generic page for any entity table
│   ├── SolverPage.tsx
│   ├── ResultsPage.tsx
│   ├── CalendarPage.tsx
│   ├── AliasPage.tsx
│   └── GuidePage.tsx
└── utils/
    ├── format.ts               # fmt, fmtPct, fmtCost, formatCell
    └── constants.ts            # NAV_ITEMS, page titles, etc.
```

## 2. NO Code Duplication — DRY Patterns

### Rust: Generic CRUD Macro

DO NOT write identical get/add/delete for each entity. Use a macro:

```rust
// commands/crud.rs
macro_rules! crud_commands {
    ($entity:ident, $collection:ident, $id_field:ident) => {
        #[tauri::command]
        pub fn paste!([<get_ $collection>])(state: State<AppState>) -> Vec<$entity> {
            state.model.lock().unwrap().$collection.clone()
        }

        #[tauri::command]
        pub fn paste!([<add_ $collection:snake>])(state: State<AppState>, item: $entity) {
            state.model.lock().unwrap().$collection.push(item);
        }

        #[tauri::command]
        pub fn paste!([<delete_ $collection:snake>])(state: State<AppState>, id: String) {
            let mut model = state.model.lock().unwrap();
            model.$collection.retain(|x| x.$id_field != id);
        }
    };
}
```

### Rust: Solver Module Separation

Current `solver.rs` is a single monolithic function. Split into:
- `variables.rs` — Creates all decision variables, returns indexed variable maps
- `constraints.rs` — Adds all constraints (one function per constraint type)
- `results.rs` — Extracts solution values into output structs

### React: Generic DataTable

DO NOT create separate table components for each entity. ONE reusable component:

```tsx
// Used like:
<DataTable 
  data={locations} 
  columns={["id", "name", "location_type", "country"]} 
  getLabel={getLabel}
/>
```

## 3. Error Handling — No Panics

### Rust Rules:
- **NEVER** use `.unwrap()` in production code except on Mutex locks
- **NEVER** use `unsafe`
- Use `Result<T, String>` for all Tauri commands
- Use `?` operator for propagation
- Provide meaningful error messages: `Err(format!("Product {} not found at location {}", pid, lid))`

### React Rules:
- Always wrap `invoke()` in try/catch
- Show errors in statusMessage, never silent failures
- Type all function parameters and returns

## 4. Naming Conventions

| Context | Convention | Example |
|---------|-----------|---------|
| Rust structs | PascalCase | `ProductLocation` |
| Rust fields | snake_case | `location_id` |
| Rust functions | snake_case | `get_yield_rate` |
| Rust modules | snake_case | `solver_types` |
| TypeScript interfaces | PascalCase | `ProductLocation` |
| TypeScript functions | camelCase | `refreshData` |
| CSS classes | kebab-case | `kpi-card` |
| Tauri commands | snake_case | `get_product_locations` |

## 5. Solver Code Quality

### Variable Indexing
- Track variables with `HashMap<(String, String, u32), usize>` — NOT linear search
- Key should be the minimal unique identifier tuple
- Example: `prod_var_idx: HashMap<(product_id, location_id, resource_id, period), usize>`

### Constraint Documentation
Every constraint must have a comment explaining the mathematical form:
```rust
// Inventory balance: Inv[t] = Inv[t-1] + production*yield + inbound - outbound - demand + unmet
// Rearranged to: Inv[t] - Inv[t-1] - production*yield - inbound + outbound + demand - unmet = 0
```

### Performance
- Pre-compute lookups into HashMaps before the main solve loop
- Avoid `.iter().find()` inside nested loops — use index maps
- Solver variable creation must be O(n), constraint building O(n*m) max

## 6. Testing Requirements

### Rust Tests
- Unit test every helper function (yield rate, consumption rate, cost lookups)
- Integration test: build a small model → solve → verify KPIs
- Test edge cases: empty model, single period, no demand

### Test File Location
```
src-tauri/src/tests/
├── model_tests.rs
├── solver_tests.rs
└── validator_tests.rs
```

## 7. Data Model Rules

### Immutability
- All model structs derive `Clone, Serialize, Deserialize`
- Solver receives `&SupplyChainModel` (immutable reference)
- Results are separate structs, never mutate the input model

### ID Strategy
- Master data: user-provided string IDs (e.g., "PLANT_TW")
- Auto-generated: UUID only for internal use where user doesn't need to reference

### Defaults
- Every struct with more than 3 fields MUST implement `Default` or a `new()` constructor
- Defaults should be safe values (0.0 for costs, true for active, 1.0 for yield)

## 8. Frontend Rules

### Component Size
- No component file > 150 lines
- No function > 50 lines
- Extract repeated JSX into components

### State Management
- Single `useModel()` hook manages all data
- Components receive data via props
- NO direct `invoke()` calls in page components — go through hooks

### Styling
- Use CSS classes, not inline styles
- Exception: truly one-off layout styles (flex gap, etc.) can be inline
- All colors from CSS variables (`var(--text-primary)`)

## 9. Documentation

### Code Comments
- Every public function has a `///` doc comment
- Complex logic gets block comments explaining WHY, not WHAT
- Solver constraints get mathematical notation

### Inline Guide
- Every new feature updates the GuidePage documentation
- New parameters get tooltips or inline help text

## 10. Git Discipline

- Each feature branch addresses ONE concern
- Commit messages: `feat:`, `fix:`, `refactor:`, `docs:`
- Never commit code that doesn't compile
