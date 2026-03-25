// OptiFlow — All enum types
// Centralized to avoid duplication and circular imports.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    Plant,
    Warehouse,
    DistributionCenter,
    Supplier,
    Customer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProductType {
    FinishedGood,
    SemiFinished,
    RawMaterial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CapacityType {
    Units,
    Hours,
    Weight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransportMode {
    Truck,
    Ocean,
    Air,
    Rail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourcingType {
    Production,
    Transport,
    CustomerAlloc,
    Procurement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DemandType {
    Forecast,
    SalesOrder,
    InterplantTransfer,
    SafetyStockReq,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReceiptType {
    PurchaseOrder,
    ProductionOrder,
    StockTransfer,
    InTransit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FenceZone {
    Frozen,
    Firm,
    Free,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CalendarType {
    ISOWeek,
    FourFourFive,
    Monthly,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Objective {
    MinimizeCost,
    MaximizeProfit,
    MaximizeDelivery,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SolveStatus {
    Optimal,
    Feasible,
    Infeasible,
    TimedOut,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}
