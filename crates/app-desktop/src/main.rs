use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use sim_core::{GameWorld, Player, MatchEvent, Vec2};
use decision_plugin::{DirectLlamaEngine, LlmEngine, DecisionContext, TacticalSettings};
use std::sync::Mutex;

fn main() {
    tracing_subscriber::fmt::init();

    App::new()
        .insert_resource(Time::<Fixed>::from_seconds(0.1)) // 10 Hz
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Eleven FC - Phase 1".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(MatchState::default())
        .insert_resource(EventLog::default())
        .insert_resource(GameWorldResource {
            world: GameWorld::new_5v5(),
        })
        .insert_resource(LlmEngineResource {
            engine: Mutex::new(None),
            is_loading: false,
            model_path: None,
        })
        .insert_resource(DecisionTimer {
            last_decision_ms: 0,
            interval_ms: 1000, // 1초 주기
        })
        .insert_resource(CameraZoom {
            level: 100.0, // 100% 기준
            base_scale: 0.0, // setup에서 계산됨
        })
        .add_systems(Startup, (setup, setup_llm_engine))
        .add_systems(
            FixedUpdate,
            (
                update_game_world,
                update_decision_loop,
            ),
        )
        .add_systems(Update, (render_hud, render_match, update_camera).chain())
        .run();
}

#[derive(Resource, Default)]
struct MatchState {
    is_running: bool,
    speed_multiplier: f32,
    time_elapsed: f32,
}

#[derive(Resource, Default)]
struct EventLog {
    events: Vec<String>,
}

#[derive(Resource)]
struct GameWorldResource {
    world: GameWorld,
}

#[derive(Resource)]
struct LlmEngineResource {
    engine: Mutex<Option<DirectLlamaEngine>>,
    is_loading: bool,
    model_path: Option<String>,
}

#[derive(Resource)]
struct DecisionTimer {
    last_decision_ms: u64,
    interval_ms: u64,
}

#[derive(Resource)]
struct CameraZoom {
    /// 줌 레벨 (100.0 = 100%, 기본값)
    level: f32,
    /// 기본 스케일 값 (100% 기준)
    base_scale: f32,
}

#[derive(Component)]
struct PlayerMarker {
    id: u32,
    team_id: u8,
}

#[derive(Component)]
struct BallMarker;

fn setup(mut commands: Commands, windows: Query<&Window>, world: Res<GameWorldResource>, mut zoom: ResMut<CameraZoom>) {
    // 경기장 크기
    let field_width = 68.0;
    let field_height = 105.0;
    
    // 경기장 중심 좌표 (월드 좌표계)
    let field_center_x = 34.0;
    let field_center_y = 52.5;
    
    // 화면 크기 가져오기
    let window = windows.single();
    let window_width = window.resolution.width();
    let window_height = window.resolution.height();
    
    // 경기장을 화면에 맞추기 위한 스케일 계산
    // Bevy 2D 카메라의 기본 높이는 100.0이므로, 경기장이 화면에 맞도록 조정
    // 화면 비율을 고려하여 더 큰 값(너비 또는 높이)을 기준으로 스케일 계산
    let aspect_ratio = window_width / window_height;
    let field_aspect = field_width / field_height;
    
    // 경기장이 더 크게 보이도록 기본 스케일 계산 (100% 기준)
    // Bevy 2D에서 scale 값이 작을수록 더 넓은 시야(더 크게 보임)
    // 기본 카메라 높이 100.0을 기준으로 경기장 크기에 맞춤
    // 2배 크게 보이도록 스케일을 절반으로 더 줄임
    let base_scale = if aspect_ratio > field_aspect {
        // 화면이 더 넓음 - 높이 기준으로 스케일 조정
        (100.0 / field_height) * 0.25 // 25%로 줄여서 2배 크게 보이도록 (100% 기준)
    } else {
        // 화면이 더 좁음 - 너비 기준으로 스케일 조정
        let effective_height = field_width / aspect_ratio;
        (100.0 / effective_height) * 0.25 // 25%로 줄여서 2배 크게 보이도록 (100% 기준)
    };
    
    // 기본 스케일을 리소스에 저장 (100% 기준)
    zoom.base_scale = base_scale;
    
    // 카메라를 경기장 중심으로 설정하고 프로젝션 조정
    let mut camera_bundle = Camera2dBundle::default();
    // 카메라 스케일 조정 (100% 기준으로 설정)
    camera_bundle.projection.scale = base_scale;
    camera_bundle.transform = Transform::from_translation(bevy::math::Vec3::new(
        field_center_x,
        field_center_y,
        100.0
    ));
    commands.spawn(camera_bundle);
    
    // 플레이어 엔티티 생성
    for player in &world.world.players {
        commands.spawn(PlayerMarker {
            id: player.id,
            team_id: player.team_id,
        });
    }
    
    // 공 엔티티 생성
    commands.spawn(BallMarker);
    
    info!("Eleven FC Phase 1 initialized with {} players", world.world.players.len());
}

/// LLM 엔진 초기화 (모델 로딩)
fn setup_llm_engine(mut llm_resource: ResMut<LlmEngineResource>) {
    // 모델 경로 설정 (환경 변수나 설정 파일에서 가져올 수 있음)
    // TODO: 실제 모델 경로로 변경
    let model_path = std::env::var("LLM_MODEL_PATH")
        .unwrap_or_else(|_| "models/qwen3-8b-q4_k_m.gguf".to_string());
    
    if std::path::Path::new(&model_path).exists() {
        info!("Loading LLM model from: {}", model_path);
        llm_resource.model_path = Some(model_path.clone());
        llm_resource.is_loading = true;
        
        // 모델 로딩은 별도 스레드에서 수행 (현재는 동기적으로)
        // TODO: 비동기 로딩 구현
        let mut engine = DirectLlamaEngine::new();
        if let Err(e) = engine.load_model(&model_path) {
            error!("Failed to load model: {}", e);
            llm_resource.is_loading = false;
        } else {
            *llm_resource.engine.lock().unwrap() = Some(engine);
            llm_resource.is_loading = false;
            info!("LLM model loaded successfully");
        }
    } else {
        warn!("Model file not found: {}. LLM decision-making will be disabled.", model_path);
    }
}

/// GameWorld에서 DecisionContext 생성
fn create_decision_context(world: &GameWorld) -> DecisionContext {
    // 최근 이벤트 변환 (최대 5개)
    let recent_events: Vec<decision_plugin::context::MatchEvent> = world.events
        .iter()
        .rev()
        .take(5)
        .map(|e| convert_match_event(e))
        .collect();
    
    // 플레이어 상태 변환
    let players: Vec<decision_plugin::context::Player> = world.players
        .iter()
        .map(|p| convert_player(p))
        .collect();
    
    // 경기 상태 변환
    let match_state = decision_plugin::context::MatchState {
        period: format!("{:?}", world.match_state.period),
        time_ms: world.match_state.time_ms,
        home_score: world.match_state.home_score as u32,
        away_score: world.match_state.away_score as u32,
    };
    
    // 현재 의도 변환
    let current_intents = world.current_intents.clone();
    
    DecisionContext {
        recent_events,
        players,
        match_state,
        current_intents,
        tactics: TacticalSettings::default(),
        current_time_ms: world.match_state.time_ms,
    }
}

fn convert_match_event(event: &MatchEvent) -> decision_plugin::context::MatchEvent {
    decision_plugin::context::MatchEvent {
        id: event.id.clone(),
        t_ms: event.t_ms,
        period: event.period.clone(),
        event_type: format!("{:?}", event.event_type),
        team_id: event.team_id.clone(),
        player_id: event.player_id.clone(),
        location: decision_plugin::intent::Vec2 {
            x: event.location.x,
            y: event.location.y,
        },
        payload: serde_json::to_value(&event.payload).unwrap_or(serde_json::json!({})),
        outcome: format!("{:?}", event.outcome),
    }
}

fn convert_player(player: &Player) -> decision_plugin::context::Player {
    decision_plugin::context::Player {
        id: player.id,
        team_id: player.team_id,
        role: player.role.clone(),
        position: decision_plugin::intent::Vec2 {
            x: player.position.x,
            y: player.position.y,
        },
        stamina: player.stamina,
        morale: player.morale,
        has_ball: player.has_ball,
        persona: decision_plugin::context::Persona {
            risk_appetite: player.persona.risk_appetite,
            pressing_intensity: player.persona.pressing_intensity,
            vision_range: player.persona.vision_range,
            patience: player.persona.patience,
            work_rate: player.persona.work_rate,
            discipline: player.persona.discipline,
            aggression: player.persona.aggression,
            confidence: player.persona.confidence,
        },
    }
}

/// 의사결정 루프 업데이트 (1초 주기)
fn update_decision_loop(
    mut world: ResMut<GameWorldResource>,
    llm_resource: Res<LlmEngineResource>,
    mut timer: ResMut<DecisionTimer>,
    match_state: Res<MatchState>,
) {
    if !match_state.is_running {
        return;
    }
    
    let current_time_ms = world.world.match_state.time_ms;
    
    // 1초 주기 체크
    if current_time_ms < timer.last_decision_ms + timer.interval_ms {
        return;
    }
    
    timer.last_decision_ms = current_time_ms;
    
    // LLM 엔진 확인
    let mut engine_guard = llm_resource.engine.lock().unwrap();
    if let Some(ref mut engine) = *engine_guard {
        if !engine.is_ready() {
            return;
        }
        
        // DecisionContext 생성
        let context = create_decision_context(&world.world);
        
        // LLM 호출
        match engine.generate_action_plan(&context) {
            Ok(action_plan) => {
                info!("Received action plan with {} intents", action_plan.intents.len());
                world.world.update_intents(action_plan.intents);
            }
            Err(e) => {
                warn!("Failed to generate action plan: {}", e);
            }
        }
    }
}

fn update_game_world(
    mut world: ResMut<GameWorldResource>,
    mut match_state: ResMut<MatchState>,
    time: Res<Time<Fixed>>,
) {
    if !match_state.is_running {
        return;
    }
    
    let delta_time = time.delta().as_secs_f32() * match_state.speed_multiplier;
    match_state.time_elapsed += delta_time;
    
    world.world.tick(delta_time);
}

fn update_camera(
    mut camera_query: Query<&mut bevy::render::camera::OrthographicProjection, With<Camera>>,
    mut transform_query: Query<&mut Transform, With<Camera>>,
    zoom: Res<CameraZoom>,
) {
    // 카메라 스케일을 줌 레벨에 따라 업데이트
    if let Ok(mut projection) = camera_query.get_single_mut() {
        // 줌 레벨에 따라 스케일 조정 (100% = base_scale, 200% = base_scale * 0.5, 50% = base_scale * 2.0)
        // 줌이 클수록(숫자가 클수록) 더 크게 보이려면 스케일을 더 작게 해야 함
        projection.scale = zoom.base_scale * (100.0 / zoom.level);
    }
    
    // 카메라는 경기장 중심에 고정 (월드 좌표계)
    if let Ok(mut camera_transform) = transform_query.get_single_mut() {
        let field_center_x = 34.0;
        let field_center_y = 52.5;
        
        // 경기장 중심에 카메라 고정
        camera_transform.translation = bevy::math::Vec3::new(
            field_center_x,
            field_center_y,
            100.0,
        );
    }
}

fn render_match(
    mut gizmos: Gizmos,
    world: Res<GameWorldResource>,
    _player_query: Query<&PlayerMarker>,
    _ball_query: Query<&BallMarker>,
) {
    // 경기장 크기
    let field_width = 68.0;
    let field_height = 105.0;
    let field_center_x = 34.0;
    let field_center_y = 52.5;
    
    // 경기장 그리기 (월드 좌표계 사용, 카메라가 중심을 보고 있으므로)
    gizmos.rect_2d(
        bevy::math::Vec2::new(field_center_x, field_center_y),
        0.0,
        bevy::math::Vec2::new(field_width, field_height),
        Color::rgb(0.2, 0.6, 0.2),
    );

    // 중앙선
    gizmos.line_2d(
        bevy::math::Vec2::new(0.0, field_center_y),
        bevy::math::Vec2::new(field_width, field_center_y),
        Color::WHITE,
    );

    // 중앙원
    gizmos.circle_2d(
        bevy::math::Vec2::new(field_center_x, field_center_y),
        9.15,
        Color::WHITE,
    );

    // 플레이어 렌더링
    for player in &world.world.players {
        let pos = bevy::math::Vec2::new(
            player.position.x,
            player.position.y,
        );
        
        let color = if player.team_id == 0 {
            Color::rgb(0.2, 0.4, 1.0) // 홈 팀 (파랑)
        } else {
            Color::rgb(1.0, 0.2, 0.2) // 어웨이 팀 (빨강)
        };
        
        gizmos.circle_2d(pos, 2.0, color);
        
        // 공을 가진 플레이어는 더 크게
        if player.has_ball {
            gizmos.circle_2d(pos, 3.0, Color::YELLOW);
        }
    }

    // 공 렌더링
    let ball_pos = bevy::math::Vec2::new(
        world.world.ball.position.x,
        world.world.ball.position.y,
    );
    gizmos.circle_2d(ball_pos, 1.5, Color::WHITE);
    
    // 공 소유자가 없으면 하얀색 테두리
    if world.world.ball.owner.is_none() {
        gizmos.circle_2d(ball_pos, 2.0, Color::WHITE);
    }
}

fn render_hud(
    mut contexts: EguiContexts,
    mut match_state: ResMut<MatchState>,
    world: Res<GameWorldResource>,
    llm_resource: Res<LlmEngineResource>,
    timer: Res<DecisionTimer>,
    mut event_log: ResMut<EventLog>,
    mut zoom: ResMut<CameraZoom>,
) {
    // 최신 이벤트를 로그에 추가
    if world.world.events.len() > event_log.events.len() {
        for event in world.world.events.iter().skip(event_log.events.len()) {
            let event_text = format!(
                "[{:.1}s] {:?} - {} -> {}",
                event.t_ms as f32 / 1000.0,
                event.event_type,
                event.player_id,
                match &event.payload {
                    sim_core::EventPayload::Pass { target_player_id, .. } => target_player_id.clone(),
                    _ => "".to_string(),
                }
            );
            event_log.events.push(event_text);
            if event_log.events.len() > 50 {
                event_log.events.remove(0);
            }
        }
    }
    
    egui::Window::new("Match Control")
        .default_pos([10.0, 10.0])
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Eleven FC Phase 1");
            
            ui.separator();
            
            if ui.button(if match_state.is_running { "Pause" } else { "Start" }).clicked() {
                match_state.is_running = !match_state.is_running;
            }
            
            ui.add_space(10.0);
            ui.label(format!("Match Time: {:.1}s", match_state.time_elapsed));
            
            ui.add_space(10.0);
            ui.label("Speed:");
            ui.add(egui::Slider::new(&mut match_state.speed_multiplier, 0.5..=2.0));
            
            ui.separator();
            ui.label(format!("Home: {} - {} :Away", world.world.match_state.home_score, world.world.match_state.away_score));
            ui.label(format!("Events: {}", world.world.events.len()));
            ui.label(format!("Active Intents: {}", world.world.current_intents.len()));
            
            ui.separator();
            ui.label("Camera Zoom:");
            let current_zoom = zoom.level;
            ui.add(egui::Slider::new(&mut zoom.level, 25.0..=400.0)
                .text(format!("{:.0}%", current_zoom))
                .show_value(true));
        });

    egui::Window::new("LLM Status")
        .default_pos([10.0, 200.0])
        .show(contexts.ctx_mut(), |ui| {
            let engine_guard = llm_resource.engine.lock().unwrap();
            if let Some(ref engine) = *engine_guard {
                ui.label("Status: Ready");
                if engine.is_ready() {
                    ui.label("✓ Model loaded");
                } else {
                    ui.label("✗ Model not ready");
                }
            } else {
                ui.label("Status: Not initialized");
            }
            
            if llm_resource.is_loading {
                ui.label("Loading model...");
            }
            
            if let Some(ref path) = llm_resource.model_path {
                ui.label(format!("Model: {}", path));
            }
            
            ui.separator();
            ui.label(format!("Last decision: {}ms ago", 
                world.world.match_state.time_ms.saturating_sub(timer.last_decision_ms)));
            ui.label(format!("Next decision in: {}ms",
                timer.interval_ms.saturating_sub(
                    world.world.match_state.time_ms.saturating_sub(timer.last_decision_ms)
                )));
        });

    egui::Window::new("Persona Settings")
        .default_pos([10.0, 400.0])
        .show(contexts.ctx_mut(), |ui| {
            if let Some(player) = world.world.players.first() {
                ui.label(format!("Player {} (Team {})", player.id, player.team_id));
                ui.label(format!("Risk Appetite: {:.2}", player.persona.risk_appetite));
                ui.label(format!("Pressing Intensity: {:.2}", player.persona.pressing_intensity));
                ui.label(format!("Vision Range: {:.1}m", player.persona.vision_range));
            }
        });

    egui::Window::new("Event Log")
        .default_pos([10.0, 600.0])
        .default_size([600.0, 200.0])
        .show(contexts.ctx_mut(), |ui| {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for event in event_log.events.iter().rev().take(20) {
                        ui.label(event);
                    }
                });
        });
}
