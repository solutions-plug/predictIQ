use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistEntry {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub status: String,
    pub source: Option<String>,
    pub referral_code: String,
    pub referred_by_code: Option<String>,
    pub position: i32,
    pub priority_score: i32,
    pub joined_at: DateTime<Utc>,
    pub invited_at: Option<DateTime<Utc>>,
    pub invitation_accepted_at: Option<DateTime<Utc>>,
    pub converted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistJoinRequest {
    pub email: String,
    pub name: Option<String>,
    #[serde(rename = "role")]
    pub user_role: Option<String>,
    #[serde(rename = "referralCode")]
    pub referral_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistJoinResponse {
    pub success: bool,
    pub position: i32,
    #[serde(rename = "referralCode")]
    pub referral_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistExportEntry {
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub status: String,
    pub position: i32,
    pub referral_code: String,
    pub referral_count: i32,
    pub joined_at: DateTime<Utc>,
    pub invited_at: Option<DateTime<Utc>>,
    pub invitation_accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistStats {
    pub total_entries: i64,
    pub pending_entries: i64,
    pub invited_entries: i64,
    pub accepted_entries: i64,
    pub total_referrals: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInviteRequest {
    pub count: Option<i32>,
    pub positions: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInviteResponse {
    pub success: bool,
    pub invited_count: i32,
    pub message: String,
}

pub fn generate_referral_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
