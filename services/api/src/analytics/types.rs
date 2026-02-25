use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackEventRequest {
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "eventData")]
    pub event_data: serde_json::Value,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackEventResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardStats {
    pub total_events: i64,
    pub unique_sessions: i64,
    pub events_by_type: Vec<EventTypeCount>,
    pub hourly_events: Vec<HourlyCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventTypeCount {
    pub event_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HourlyCount {
    pub hour: DateTime<Utc>,
    pub count: i64,
}
