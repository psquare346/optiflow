// OptiFlow — Calendar & time fence types

use serde::{Deserialize, Serialize};
use super::enums::*;

// ─── Planning Calendar ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningCalendar {
    pub id: String,
    pub name: String,
    pub calendar_type: CalendarType,
    pub start_date: String,
    pub buckets: Vec<PlanningBucket>,
}

impl PlanningCalendar {
    pub fn new(id: &str, name: &str, cal_type: CalendarType, start_date: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            calendar_type: cal_type,
            start_date: start_date.to_string(),
            buckets: Vec::new(),
        }
    }
}

// ─── Planning Bucket (one period in the calendar) ───────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningBucket {
    pub period: u32,
    pub label: String,
    pub start_date: String,
    pub end_date: String,
    pub working_days: u32,
    pub is_working: bool,
    pub fence_zone: FenceZone,
}

// ─── Calendar Entry (capacity overrides by period) ──────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEntry {
    pub location_id: String,
    pub resource_id: Option<String>,
    pub period: u32,
    pub available_capacity: Option<f64>,
    pub is_working: bool,
    pub shift_factor: f64,
}

// ─── Legacy TimeBucket (kept for backward compat) ───────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBucket {
    pub period: u32,
    pub start_date: String,
    pub end_date: String,
    pub bucket_type: String,
    pub working_days: u32,
}
