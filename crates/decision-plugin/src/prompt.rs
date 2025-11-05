use crate::context::DecisionContext;

/// 프롬프트 생성기
pub struct PromptGenerator;

impl PromptGenerator {
    /// DecisionContext를 LLM 프롬프트로 변환
    pub fn generate_prompt(context: &DecisionContext) -> String {
        let mut prompt = String::new();
        
        // 시스템 프롬프트
        prompt.push_str("You are a tactical decision engine for a 5v5 football simulation.\n");
        prompt.push_str("Your task is to determine actions for 10 players based on the current match situation.\n");
        prompt.push_str("For each player, decide whether to CONTINUE their current action or assign a NEW action.\n\n");
        
        // 경기 상태
        prompt.push_str("## Match State\n");
        prompt.push_str(&format!("Time: {}ms (Period: {:?})\n", context.current_time_ms, context.match_state.period));
        prompt.push_str(&format!("Score: Home {} - {} Away\n\n", 
            context.match_state.home_score, context.match_state.away_score));
        
        // 선수 상태
        prompt.push_str("## Players\n");
        for player in &context.players {
            let ball_status = if player.has_ball { "HAS_BALL" } else { "NO_BALL" };
            prompt.push_str(&format!(
                "Player {} (Team {}): Position ({:.1}, {:.1}), Stamina {:.2}, Morale {:.2}, {}\n",
                player.id, player.team_id, 
                player.position.x, player.position.y,
                player.stamina, player.morale, ball_status
            ));
        }
        prompt.push_str("\n");
        
        // 최근 이벤트
        if !context.recent_events.is_empty() {
            prompt.push_str("## Recent Events\n");
            for event in context.recent_events.iter().take(5) {
                // 이벤트 페이로드 파싱 (간단한 텍스트 표현)
                let event_desc = if let Some(payload_obj) = event.payload.as_object() {
                    if let Some(target_id) = payload_obj.get("target_player_id").and_then(|v| v.as_str()) {
                        format!("Pass to {}", target_id)
                    } else if let Some(scorer_id) = payload_obj.get("scorer_id").and_then(|v| v.as_str()) {
                        format!("GOAL by {}", scorer_id)
                    } else {
                        "Other event".to_string()
                    }
                } else {
                    "Other event".to_string()
                };
                
                prompt.push_str(&format!(
                    "[{}ms] {} - Player {} - {}\n",
                    event.t_ms, event.event_type, event.player_id, event_desc
                ));
            }
            prompt.push_str("\n");
        }
        
        // 현재 의도
        if !context.current_intents.is_empty() {
            prompt.push_str("## Current Intents\n");
            for intent in &context.current_intents {
                let action_desc = if let Some(action) = &intent.action {
                    format!("{:?}", action)
                } else {
                    "None".to_string()
                };
                prompt.push_str(&format!(
                    "Player {}: Status {:?}, Action: {}\n",
                    intent.player_id, intent.status, action_desc
                ));
            }
            prompt.push_str("\n");
        }
        
        // 전술 설정
        prompt.push_str("## Tactical Settings\n");
        prompt.push_str(&format!("Attack/Defense Balance: {:.2}\n", context.tactics.attack_defense_balance));
        prompt.push_str(&format!("Pressing Intensity: {:.2}\n", context.tactics.pressing_intensity));
        if !context.tactics.player_roles.is_empty() {
            prompt.push_str("Player Roles:\n");
            for role in &context.tactics.player_roles {
                prompt.push_str(&format!("  Player {}: {}\n", role.player_id, role.role_name));
            }
        }
        prompt.push_str("\n");
        
        // 출력 형식 지시
        prompt.push_str("## Your Task\n");
        prompt.push_str("Generate a JSON response with actions for all 10 players.\n");
        prompt.push_str("Format:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"intents\": [\n");
        prompt.push_str("    {\n");
        prompt.push_str("      \"player_id\": <number>,\n");
        prompt.push_str("      \"status\": \"New\" or \"Continue\",\n");
        prompt.push_str("      \"action\": {\n");
        prompt.push_str("        \"type\": \"AttackSpace\" | \"MarkPlayer\" | \"FindPassOption\" | \"HoldPosition\" | \"Press\" | \"MoveToBall\" | \"ReturnToPosition\" | \"BlockSpace\",\n");
        prompt.push_str("        \"target\": {\"x\": <number>, \"y\": <number>} (if applicable),\n");
        prompt.push_str("        \"target_id\": <number> (if applicable)\n");
        prompt.push_str("      } (only if status is \"New\")\n");
        prompt.push_str("    }\n");
        prompt.push_str("  ]\n");
        prompt.push_str("}\n");
        prompt.push_str("\n");
        prompt.push_str("Important:\n");
        prompt.push_str("- Use \"Continue\" when the current action is still valid\n");
        prompt.push_str("- Use \"New\" when a new action is needed\n");
        prompt.push_str("- Include all 10 players in the response\n");
        prompt.push_str("- Actions should be tactical and context-aware\n");
        
        prompt
    }
    
    /// JSON 응답을 ActionPlan으로 파싱
    pub fn parse_response(response: &str, generated_at_ms: u64, latency_ms: u64) -> Result<crate::intent::ActionPlan, String> {
        // JSON 파싱 시도
        let json: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        let intents_array = json.get("intents")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Missing 'intents' array".to_string())?;
        
        let mut intents = Vec::new();
        for intent_json in intents_array {
            let player_id = intent_json.get("player_id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Missing player_id".to_string())? as u32;
            
            let status_str = intent_json.get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing status".to_string())?;
            
            let status = match status_str.to_lowercase().as_str() {
                "new" => crate::intent::IntentStatus::New,
                "continue" => crate::intent::IntentStatus::Continue,
                _ => return Err(format!("Invalid status: {}", status_str)),
            };
            
            let action = if status == crate::intent::IntentStatus::New {
                intent_json.get("action")
                    .and_then(|a| Self::parse_action(a))
            } else {
                None
            };
            
            intents.push(crate::intent::Intent::new(
                player_id,
                status,
                action,
                generated_at_ms,
            ));
        }
        
        Ok(crate::intent::ActionPlan::new(intents, generated_at_ms, latency_ms))
    }
    
    fn parse_action(action_json: &serde_json::Value) -> Option<crate::intent::Action> {
        let action_type = action_json.get("type")?.as_str()?;
        
        match action_type {
            "AttackSpace" => {
                let target = action_json.get("target")?;
                Some(crate::intent::Action::AttackSpace {
                    target: crate::intent::Vec2 {
                        x: target.get("x")?.as_f64()? as f32,
                        y: target.get("y")?.as_f64()? as f32,
                    },
                })
            }
            "MarkPlayer" => {
                Some(crate::intent::Action::MarkPlayer {
                    target_id: action_json.get("target_id")?.as_u64()? as u32,
                })
            }
            "FindPassOption" => Some(crate::intent::Action::FindPassOption),
            "HoldPosition" => Some(crate::intent::Action::HoldPosition),
            "Press" => {
                let target = action_json.get("target")?;
                Some(crate::intent::Action::Press {
                    target: crate::intent::Vec2 {
                        x: target.get("x")?.as_f64()? as f32,
                        y: target.get("y")?.as_f64()? as f32,
                    },
                })
            }
            "MoveToBall" => Some(crate::intent::Action::MoveToBall),
            "ReturnToPosition" => {
                let position = action_json.get("position")?;
                Some(crate::intent::Action::ReturnToPosition {
                    position: crate::intent::Vec2 {
                        x: position.get("x")?.as_f64()? as f32,
                        y: position.get("y")?.as_f64()? as f32,
                    },
                })
            }
            "BlockSpace" => {
                let target = action_json.get("target")?;
                Some(crate::intent::Action::BlockSpace {
                    target: crate::intent::Vec2 {
                        x: target.get("x")?.as_f64()? as f32,
                        y: target.get("y")?.as_f64()? as f32,
                    },
                })
            }
            _ => None,
        }
    }
}

