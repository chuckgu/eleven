use serde::{Deserialize, Serialize};

/// 2D 벡터 (decision-plugin 전용, sim-core와 호환)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 의도 상태
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentStatus {
    /// 새로운 의도
    New,
    /// 이전 의도 유지
    Continue,
    /// 대기 상태 (의도 없음)
    Idle,
}

/// 행동 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// 공간으로 침투
    AttackSpace {
        target: crate::intent::Vec2,
    },
    /// 특정 플레이어 마크
    MarkPlayer {
        target_id: u32,
    },
    /// 패스 옵션 찾기
    FindPassOption,
    /// 위치 유지
    HoldPosition,
    /// 압박
    Press {
        target: crate::intent::Vec2,
    },
    /// 공을 향해 이동
    MoveToBall,
    /// 기본 포지션 복귀
    ReturnToPosition {
        position: crate::intent::Vec2,
    },
    /// 공간을 막기
    BlockSpace {
        target: crate::intent::Vec2,
    },
}

/// 선수 의도
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub player_id: u32,
    pub status: IntentStatus,
    pub action: Option<Action>,
    /// 의도 생성 시점 (ms)
    pub created_at_ms: u64,
    /// 의도 지속 시간 (ms, None이면 계속)
    pub duration_ms: Option<u64>,
}

impl Intent {
    pub fn new(player_id: u32, status: IntentStatus, action: Option<Action>, created_at_ms: u64) -> Self {
        Self {
            player_id,
            status,
            action,
            created_at_ms,
            duration_ms: None,
        }
    }

    pub fn is_expired(&self, current_time_ms: u64) -> bool {
        if let Some(duration) = self.duration_ms {
            current_time_ms > self.created_at_ms + duration
        } else {
            false
        }
    }
}

/// 10명 선수의 액션 플랜
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub intents: Vec<Intent>,
    /// LLM 호출 시점 (ms)
    pub generated_at_ms: u64,
    /// LLM 생성 지연 (ms)
    pub latency_ms: u64,
}

impl ActionPlan {
    pub fn new(intents: Vec<Intent>, generated_at_ms: u64, latency_ms: u64) -> Self {
        Self {
            intents,
            generated_at_ms,
            latency_ms,
        }
    }

    pub fn get_intent(&self, player_id: u32) -> Option<&Intent> {
        self.intents.iter().find(|i| i.player_id == player_id)
    }
}

