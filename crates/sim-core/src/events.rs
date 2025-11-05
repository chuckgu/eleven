use serde::{Deserialize, Serialize};
use crate::types::Vec2;

/// 경기 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvent {
    pub id: String,
    pub t_ms: u64,
    pub period: String, // "H1", "H2"
    pub event_type: EventType,
    pub team_id: String,
    pub player_id: String,
    pub location: Vec2,
    pub payload: EventPayload,
    pub outcome: EventOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventType {
    Pass,
    Shot,
    Tackle,
    Interception,
    Press,
    Turnover,
    SetPiece,
    Foul,
    Save,
    Goal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPayload {
    Pass {
        target_player_id: String,
        distance: f32,
        risk: f32,
    },
    Shot {
        distance: f32,
        angle: f32,
        on_target: bool,
    },
    Tackle {
        on_player_id: String,
        successful: bool,
    },
    Goal {
        scorer_id: String,
        assist_id: Option<String>,
    },
    Empty,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EventOutcome {
    Complete,
    Incomplete,
    Success,
    Failure,
}

/// 해설
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commentary {
    pub event_id: String,
    pub text: String,
    pub tone: Tone,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Tone {
    Calm,
    Aggressive,
    Excited,
}

/// 하이라이트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub event_id: String,
    pub status: HighlightStatus,
    pub prompt: String,
    pub image_path: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HighlightStatus {
    Queued,
    Rendering,
    Done,
    Error,
}

