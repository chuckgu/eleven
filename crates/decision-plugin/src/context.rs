use serde::{Deserialize, Serialize};
use crate::intent::Intent;

// sim-core 타입들을 재정의 (순환 의존성 방지)
// 실제 사용 시 serde로 직렬화/역직렬화하여 전달
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvent {
    pub id: String,
    pub t_ms: u64,
    pub period: String,
    pub event_type: String,
    pub team_id: String,
    pub player_id: String,
    pub location: crate::intent::Vec2,
    pub payload: serde_json::Value,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub team_id: u8,
    pub role: String,
    pub position: crate::intent::Vec2,
    pub stamina: f32,
    pub morale: f32,
    pub has_ball: bool,
    pub persona: Persona,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub risk_appetite: f32,
    pub pressing_intensity: f32,
    pub vision_range: f32,
    pub patience: f32,
    pub work_rate: f32,
    pub discipline: f32,
    pub aggression: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchState {
    pub period: String,
    pub time_ms: u64,
    pub home_score: u32,
    pub away_score: u32,
}

/// LLM 의사결정을 위한 컨텍스트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    /// 최근 이벤트 (최대 N개)
    pub recent_events: Vec<MatchEvent>,
    /// 현재 선수 상태들
    pub players: Vec<Player>,
    /// 경기 상태
    pub match_state: MatchState,
    /// 현재 의도들 (이전 LLM 호출 결과)
    pub current_intents: Vec<Intent>,
    /// 전술 설정
    pub tactics: TacticalSettings,
    /// 현재 시간 (ms)
    pub current_time_ms: u64,
}

/// 전술 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalSettings {
    /// 공격/수비 밸런스 (0.0 = 수비, 1.0 = 공격)
    pub attack_defense_balance: f32,
    /// 압박 높이 (0.0 = 낮음, 1.0 = 높음)
    pub pressing_intensity: f32,
    /// 선수 역할 할당
    pub player_roles: Vec<PlayerRole>,
}

/// 선수 역할
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRole {
    pub player_id: u32,
    pub role_name: String, // "Target Man", "Ball-Playing Defender", etc.
}

impl Default for TacticalSettings {
    fn default() -> Self {
        Self {
            attack_defense_balance: 0.5,
            pressing_intensity: 0.5,
            player_roles: Vec::new(),
        }
    }
}

