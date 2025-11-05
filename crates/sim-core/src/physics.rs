use crate::types::{Vec2, Ball, Player};

/// 물리 상수
pub const FIELD_WIDTH: f32 = 68.0;
pub const FIELD_HEIGHT: f32 = 105.0;
pub const PLAYER_RADIUS: f32 = 0.5;
pub const BALL_RADIUS: f32 = 0.11;
pub const MIN_DISTANCE: f32 = 1.0; // 최소 충돌 회피 거리

/// 이동 계산
pub fn move_towards(position: Vec2, target: Vec2, max_speed: f32, delta_time: f32) -> Vec2 {
    let direction = Vec2::new(target.x - position.x, target.y - position.y);
    let distance = direction.length();
    
    if distance < 0.01 {
        return position;
    }
    
    let normalized = direction.normalize();
    let move_distance = (max_speed * delta_time).min(distance);
    
    Vec2::new(
        position.x + normalized.x * move_distance,
        position.y + normalized.y * move_distance,
    )
}

/// 충돌 회피
pub fn avoid_collision(
    position: Vec2,
    other_positions: &[Vec2],
    min_distance: f32,
) -> Vec2 {
    let mut avoidance = Vec2::new(0.0, 0.0);
    let mut count = 0;
    
    for other in other_positions {
        let distance = position.distance(other);
        if distance < min_distance && distance > 0.01 {
            let direction = Vec2::new(
                position.x - other.x,
                position.y - other.y,
            ).normalize();
            let strength = 1.0 - (distance / min_distance);
            avoidance.x += direction.x * strength;
            avoidance.y += direction.y * strength;
            count += 1;
        }
    }
    
    if count > 0 {
        avoidance.x /= count as f32;
        avoidance.y /= count as f32;
        let normalized = avoidance.normalize();
        return Vec2::new(
            position.x + normalized.x * 0.5,
            position.y + normalized.y * 0.5,
        );
    }
    
    position
}

/// 공 소유권 체크
pub fn check_ball_ownership(
    ball: &Ball,
    players: &[Player],
    possession_range: f32,
) -> Option<u32> {
    for player in players {
        let distance = ball.position.distance(&player.position);
        if distance < possession_range {
            return Some(player.id);
        }
    }
    None
}

/// 공 이동 (물리 업데이트)
pub fn update_ball(
    ball: &mut Ball,
    delta_time: f32,
    friction: f32,
) {
    // 속도 감쇠 (마찰)
    ball.velocity.x *= 1.0 - (friction * delta_time);
    ball.velocity.y *= 1.0 - (friction * delta_time);
    
    // 위치 업데이트
    ball.position.x += ball.velocity.x * delta_time;
    ball.position.y += ball.velocity.y * delta_time;
    
    // 경기장 경계 체크
    ball.position.x = ball.position.x.max(0.0).min(FIELD_WIDTH);
    ball.position.y = ball.position.y.max(0.0).min(FIELD_HEIGHT);
}

