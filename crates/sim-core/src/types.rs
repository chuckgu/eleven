use serde::{Deserialize, Serialize};

/// 플레이어 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub team_id: u8,
    pub role: String,
    pub position: Vec2,
    pub stamina: f32,
    pub morale: f32,
    pub has_ball: bool,
    pub persona: Persona,
}

/// 2D 벡터
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: &Vec2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Vec2 {
        let len = self.length();
        if len > 0.0 {
            Vec2::new(self.x / len, self.y / len)
        } else {
            Vec2::new(0.0, 0.0)
        }
    }
}

/// 공 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ball {
    pub position: Vec2,
    pub velocity: Vec2,
    pub owner: Option<u32>, // 플레이어 ID
}

impl Ball {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            velocity: Vec2::new(0.0, 0.0),
            owner: None,
        }
    }
}

/// 페르소나 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// 위험 선호도 (0.0 = 매우 안전, 1.0 = 매우 공격적)
    pub risk_appetite: f32,
    /// 압박 강도 (0.0 = 수동적, 1.0 = 적극적)
    pub pressing_intensity: f32,
    /// 시야 범위 (미터)
    pub vision_range: f32,
    /// 인내심 (0.0 = 성급함, 1.0 = 침착함)
    pub patience: f32,
    /// 작업량 (0.0 = 최소, 1.0 = 최대)
    pub work_rate: f32,
    /// 패턴 선호도
    pub pattern_preference: PatternPreference,
    /// 규율 (0.0 = 무질서, 1.0 = 엄격)
    pub discipline: f32,
    /// 공격성 (0.0 = 온화, 1.0 = 격렬)
    pub aggression: f32,
    /// 자신감 (0.0 = 낮음, 1.0 = 높음)
    pub confidence: f32,
}

impl Default for Persona {
    fn default() -> Self {
        Self {
            risk_appetite: 0.5,
            pressing_intensity: 0.5,
            vision_range: 15.0,
            patience: 0.5,
            work_rate: 0.5,
            pattern_preference: PatternPreference::default(),
            discipline: 0.5,
            aggression: 0.5,
            confidence: 0.5,
        }
    }
}

/// 패턴 선호도
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternPreference {
    /// 사이드 플레이 전환 (0.0 ~ 1.0)
    pub switch_play: f32,
    /// 스루 패스 (0.0 ~ 1.0)
    pub through_ball: f32,
    /// 컷백 (0.0 ~ 1.0)
    pub cut_back: f32,
}

impl Default for PatternPreference {
    fn default() -> Self {
        Self {
            switch_play: 0.5,
            through_ball: 0.5,
            cut_back: 0.5,
        }
    }
}

/// 전술 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tactics {
    pub formation: Formation,
    pub roles: Vec<Role>,
    pub triggers: Vec<Trigger>,
}

/// 포메이션 (5v5 예시)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Formation {
    /// 2-1-1 (수비 2, 미드필더 1, 공격 1)
    TwoOneOne,
    /// 3-1-0 (수비 3, 미드필더 1)
    ThreeOneZero,
    /// 2-2-0 (수비 2, 미드필더 2)
    TwoTwoZero,
}

/// 역할
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub zone: Zone,
    pub behavior: Behavior,
}

/// 영역
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Zone {
    Defense,
    Midfield,
    Attack,
}

/// 행동
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Behavior {
    Hold,
    Press,
    Overlap,
}

/// 트리거
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    PressOnLoss,
    OverlapLeft,
    OverlapRight,
}

/// 경기 상태
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchState {
    pub period: Period,
    pub time_ms: u64,
    pub home_score: u8,
    pub away_score: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Period {
    H1,
    H2,
    ExtraTime,
}

