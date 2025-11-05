use crate::types::*;
use crate::physics::*;
use crate::decision::*;
use crate::events::*;
use decision_plugin::{Intent, Action, IntentStatus};

/// 게임 월드 상태
#[derive(Debug, Clone)]
pub struct GameWorld {
    pub players: Vec<Player>,
    pub ball: Ball,
    pub match_state: MatchState,
    pub events: Vec<MatchEvent>,
    /// 현재 플레이어 의도들 (LLM에서 생성)
    pub current_intents: Vec<Intent>,
}

impl GameWorld {
    pub fn new_5v5() -> Self {
        let mut players = Vec::new();
        
        // 홈 팀 (5명) - 수직 배치
        let home_positions = vec![
            Vec2::new(10.0, 10.0),   // 수비
            Vec2::new(10.0, 20.0),   // 수비
            Vec2::new(20.0, 40.0),   // 미드필더
            Vec2::new(30.0, 60.0),   // 공격
            Vec2::new(50.0, 80.0),   // 공격
        ];
        
        for (i, pos) in home_positions.iter().enumerate() {
            let mut persona = Persona::default();
            persona.risk_appetite = 0.3 + (i as f32 * 0.1);
            
            players.push(Player {
                id: i as u32,
                team_id: 0,
                role: "Player".to_string(),
                position: *pos,
                stamina: 1.0,
                morale: 0.7,
                has_ball: false,
                persona,
            });
        }
        
        // 어웨이 팀 (5명)
        let away_positions = vec![
            Vec2::new(58.0, 95.0),   // 수비
            Vec2::new(58.0, 85.0),   // 수비
            Vec2::new(48.0, 65.0),   // 미드필더
            Vec2::new(38.0, 45.0),   // 공격
            Vec2::new(18.0, 25.0),   // 공격
        ];
        
        for (i, pos) in away_positions.iter().enumerate() {
            let mut persona = Persona::default();
            persona.risk_appetite = 0.4 + (i as f32 * 0.1);
            persona.pressing_intensity = 0.6;
            
            players.push(Player {
                id: (i + 5) as u32,
                team_id: 1,
                role: "Player".to_string(),
                position: *pos,
                stamina: 1.0,
                morale: 0.7,
                has_ball: false,
                persona,
            });
        }
        
        let mut ball = Ball::new(FIELD_WIDTH / 2.0, FIELD_HEIGHT / 2.0);
        
        // 시작 시 첫 번째 홈 팀 플레이어에게 공 배치
        if let Some(first_player) = players.first_mut() {
            first_player.has_ball = true;
            ball.owner = Some(first_player.id);
            ball.position = first_player.position;
        }
        
        Self {
            players,
            ball,
            match_state: MatchState {
                period: Period::H1,
                time_ms: 0,
                home_score: 0,
                away_score: 0,
            },
            events: Vec::new(),
            current_intents: Vec::new(),
        }
    }
    
    /// 의도 업데이트 (LLM에서 생성된 ActionPlan에서 호출)
    pub fn update_intents(&mut self, intents: Vec<Intent>) {
        // 기존 의도와 새 의도 병합
        for new_intent in intents {
            // NEW 상태면 기존 의도 교체, CONTINUE면 유지
            if new_intent.status == IntentStatus::New {
                self.current_intents.retain(|i| i.player_id != new_intent.player_id);
                self.current_intents.push(new_intent);
            } else if new_intent.status == IntentStatus::Continue {
                // CONTINUE는 기존 의도 유지 (이미 있으면 무시)
                if !self.current_intents.iter().any(|i| i.player_id == new_intent.player_id) {
                    // 기존 의도가 없으면 새로 추가 (이전 의도 유지)
                    self.current_intents.push(new_intent);
                }
            }
            // IDLE 상태는 기존 의도 제거
            else if new_intent.status == IntentStatus::Idle {
                self.current_intents.retain(|i| i.player_id != new_intent.player_id);
            }
        }
    }
    
    /// 플레이어의 의도에 따른 목표 위치 계산
    fn get_target_from_intent(
        &self,
        player_id: u32,
        intent: &Intent,
    ) -> Option<Vec2> {
        match &intent.action {
            Some(Action::AttackSpace { target }) => Some(Vec2::new(target.x, target.y)),
            Some(Action::Press { target }) => Some(Vec2::new(target.x, target.y)),
            Some(Action::MoveToBall) => Some(self.ball.position),
            Some(Action::ReturnToPosition { position }) => Some(Vec2::new(position.x, position.y)),
            Some(Action::BlockSpace { target }) => Some(Vec2::new(target.x, target.y)),
            Some(Action::MarkPlayer { target_id }) => {
                // 대상 플레이어 위치 찾기
                self.players.iter()
                    .find(|p| p.id == *target_id)
                    .map(|p| p.position)
            }
            Some(Action::FindPassOption) | Some(Action::HoldPosition) => {
                // 현재 위치 유지
                self.players.iter()
                    .find(|p| p.id == player_id)
                    .map(|p| p.position)
            }
            None => None,
        }
    }
    
    /// 게임 틱 업데이트 (10Hz)
    pub fn tick(&mut self, delta_time: f32) {
        self.match_state.time_ms += (delta_time * 1000.0) as u64;
        
        // 만료된 의도 제거
        self.current_intents.retain(|intent| {
            !intent.is_expired(self.match_state.time_ms)
        });
        
        // 1. 의도 기반 플레이어 이동
        let current_positions: Vec<Vec2> = self.players.iter().map(|p| p.position).collect();
        let mut new_positions = Vec::new();
        
        for (i, player) in self.players.iter().enumerate() {
            let target = if let Some(intent) = self.current_intents.iter()
                .find(|intent| intent.player_id == player.id)
            {
                // 의도가 있으면 의도에 따라 목표 결정
                if let Some(target_pos) = self.get_target_from_intent(player.id, intent) {
                    target_pos
                } else {
                    // 의도가 있지만 목표를 계산할 수 없으면 현재 위치 유지
                    player.position
                }
            } else {
                // 의도가 없으면 기본 동작 (공을 향해 이동하거나 위치 복귀)
                if self.ball.owner.is_none() {
                    self.ball.position
                } else {
                    // 기본 위치로 복귀
                    let base_x = if player.team_id == 0 { 10.0 } else { 58.0 };
                    Vec2::new(base_x, player.position.y)
                }
            };
            
            let max_speed = 5.0 * player.persona.work_rate;
            let other_positions: Vec<Vec2> = current_positions.iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, pos)| *pos)
                .collect();
            
            let new_pos = move_towards(player.position, target, max_speed, delta_time);
            new_positions.push(avoid_collision(new_pos, &other_positions, MIN_DISTANCE));
        }
        
        // 위치 업데이트
        for (player, new_pos) in self.players.iter_mut().zip(new_positions.iter()) {
            if !player.has_ball {
                player.position = *new_pos;
            }
        }
        
        // 2. 공 소유권 업데이트
        if let Some(owner_id) = check_ball_ownership(&self.ball, &self.players, 1.5) {
            self.ball.owner = Some(owner_id);
            for player in &mut self.players {
                player.has_ball = player.id == owner_id;
                if player.has_ball {
                    player.position = self.ball.position;
                }
            }
        } else {
            // 공이 자유롭게 움직임
            update_ball(&mut self.ball, delta_time, 0.95);
        }
        
        // 3. 의도 기반 특수 동작 처리 (FindPassOption 등)
        // TODO: 패스, 슈팅 등 고급 액션 처리
    }
    
    /// 홈 팀 플레이어들
    pub fn home_players(&self) -> Vec<&Player> {
        self.players.iter().filter(|p| p.team_id == 0).collect()
    }
    
    /// 어웨이 팀 플레이어들
    pub fn away_players(&self) -> Vec<&Player> {
        self.players.iter().filter(|p| p.team_id == 1).collect()
    }
}
