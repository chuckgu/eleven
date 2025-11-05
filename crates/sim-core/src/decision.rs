use crate::types::{Player, Persona, Vec2};

/// 행동 타입
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Hold,           // 드리블/보존
    PassSafe,       // 안전한 패스
    PassRisk,       // 위험한 패스
    Shoot,          // 슈팅
    Press,          // 압박
    ReturnPosition, // 위치 복귀
    Cover,          // 커버
}

/// 행동과 유틸리티 점수
#[derive(Debug, Clone)]
pub struct ActionScore {
    pub action: Action,
    pub utility: f32,
}

/// 유틸리티 계산 함수들

/// 패스 유틸리티
pub fn pass_utility(
    player: &Player,
    target: &Vec2,
    _teammates: &[Player],
    opponents: &[Player],
    persona: &Persona,
) -> f32 {
    let distance = player.position.distance(target);
    let max_distance = persona.vision_range;
    
    // 거리 기반 점수
    let distance_score = if distance > max_distance {
        0.0
    } else {
        1.0 - (distance / max_distance)
    };
    
    // 위험 선호도 반영
    let risk_factor = if distance > max_distance * 0.7 {
        persona.risk_appetite
    } else {
        1.0 - persona.risk_appetite
    };
    
    // 상대방 차단 위험도 계산
    let blocking_risk = calculate_blocking_risk(&player.position, target, opponents);
    let blocking_score = 1.0 - blocking_risk;
    
    // 전진 이득 (골대 방향으로 갈수록 높음)
    let goal_direction = Vec2::new(FIELD_WIDTH / 2.0, FIELD_HEIGHT);
    let forward_gain = calculate_forward_gain(&player.position, target, &goal_direction);
    
    distance_score * risk_factor * blocking_score * (0.7 + 0.3 * forward_gain)
}

/// 슈팅 유틸리티
pub fn shoot_utility(
    player: &Player,
    goal_position: &Vec2,
    opponents: &[Player],
    persona: &Persona,
) -> f32 {
    let distance = player.position.distance(goal_position);
    let max_shoot_distance = 20.0;
    
    if distance > max_shoot_distance {
        return 0.0;
    }
    
    // 거리 점수
    let distance_score = 1.0 - (distance / max_shoot_distance);
    
    // 각도 점수 (간단화: 골대 중심에 가까울수록 높음)
    let angle_score = 1.0;
    
    // 자신감 반영
    let confidence_factor = persona.confidence;
    
    // 수비 압박 계산
    let defensive_pressure = calculate_defensive_pressure(&player.position, opponents);
    let pressure_score = 1.0 - defensive_pressure;
    
    distance_score * angle_score * confidence_factor * pressure_score
}

/// 압박 유틸리티
pub fn press_utility(
    player: &Player,
    target: &Vec2,
    _opponents: &[Player],
    persona: &Persona,
    stamina: f32,
) -> f32 {
    let distance = player.position.distance(target);
    let max_press_distance = 5.0;
    
    if distance > max_press_distance {
        return 0.0;
    }
    
    // 거리 점수
    let distance_score = 1.0 - (distance / max_press_distance);
    
    // 압박 강도 반영
    let pressing_factor = persona.pressing_intensity;
    
    // 체력 반영
    let stamina_factor = stamina;
    
    distance_score * pressing_factor * stamina_factor
}

/// 최고 유틸리티 행동 선택
pub fn select_best_action(scores: &[ActionScore]) -> Option<Action> {
    scores
        .iter()
        .max_by(|a, b| a.utility.partial_cmp(&b.utility).unwrap())
        .map(|s| s.action)
}

// 헬퍼 함수들
fn calculate_blocking_risk(
    from: &Vec2,
    to: &Vec2,
    opponents: &[Player],
) -> f32 {
    let mut risk: f32 = 0.0;
    for opponent in opponents {
        let dist_to_line = distance_to_line_segment(from, to, &opponent.position);
        if dist_to_line < 2.0 {
            risk += 0.3;
        }
    }
    risk.min(1.0f32)
}

fn calculate_forward_gain(
    from: &Vec2,
    to: &Vec2,
    goal: &Vec2,
) -> f32 {
    let from_dist = from.distance(goal);
    let to_dist = to.distance(goal);
    if from_dist > 0.0 {
        ((from_dist - to_dist) / from_dist).max(0.0f32).min(1.0f32)
    } else {
        0.0
    }
}

fn calculate_defensive_pressure(
    position: &Vec2,
    opponents: &[Player],
) -> f32 {
    let mut pressure = 0.0;
    for opponent in opponents {
        let dist = position.distance(&opponent.position);
        if dist < 5.0 {
            pressure += (1.0 - dist / 5.0) * 0.3;
        }
    }
    pressure.min(1.0f32)
}

fn distance_to_line_segment(a: &Vec2, b: &Vec2, p: &Vec2) -> f32 {
    let ab = Vec2::new(b.x - a.x, b.y - a.y);
    let ap = Vec2::new(p.x - a.x, p.y - a.y);
    let ab_squared = ab.x * ab.x + ab.y * ab.y;
    
    if ab_squared < 0.01 {
        return a.distance(p);
    }
    
    let t = ((ap.x * ab.x + ap.y * ab.y) / ab_squared).max(0.0).min(1.0);
    let closest = Vec2::new(a.x + t * ab.x, a.y + t * ab.y);
    closest.distance(p)
}

const FIELD_WIDTH: f32 = 68.0;
const FIELD_HEIGHT: f32 = 105.0;

