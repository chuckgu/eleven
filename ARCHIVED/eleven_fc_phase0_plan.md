# Eleven FC — Phase 0 구현 계획서 (로컬 LLM 우선)

## 1. 목표와 성공 기준
- **핵심 목표**: 2D 시뮬(5v5~7v7) + 파라미터형 페르소나 + 이벤트→문장 해설 + 하이라이트 스틸 비동기 생성 + 간단 대시보드를 0~3개월 내 수직 슬라이스로 완성.
- **성공 기준**
  - 전술/페르소나 차이가 관찰 가능(위험 선호, 압박 강도, 시야 등 행동 차별화 체감).
  - 10 Hz 의사결정 루프에서 안정적 경기 전개(충돌/패스/슈팅/가로채기/세트피스 처리).
  - 이벤트→NLG 해설이 실시간 감상에 충분한 몰입 제공(지연 < 500ms/문장, 톤 제어 동작).
  - 하이라이트 스틸이 경기 중 주요 이벤트를 비동기로 10초 내 생성, 갤러리에 자동 축적.
  - 대시보드에서 경기 진행/속도/전술/페르소나 슬라이더 조정, 이벤트/해설/하이라이트 확인 가능.

## 2. 범위(스코프) / 비범위(논스코프)
- **범위**
  - 2D 탑다운 경기 엔진(5v5~7v7), 10 Hz 의사결정.
  - 파라미터형 페르소나(규칙/유틸리티 기반)와 기본 전술 캔버스(포메이션/역할/트리거).
  - 이벤트 스키마 정의 및 이벤트→문장 해설(NLG) 파이프라인.
  - 하이라이트 스틸(이미지) 비동기 생성 파이프라인.
  - 로컬 LLM 우선 아키텍처 및 간단 데스크톱 대시보드(Bevy/egui) 제공.
- **비범위**
  - 실시간 방송급 생성형 비디오, 3D 렌더링, 대규모 온라인 리그 운영.
  - 대규모 RL/미분가능 훈련 루프, 고급 전술 분석, 심층 데이터 통합.

## 3. 기술 선택(로컬 우선)
- **시뮬 코어/애플리케이션**: Rust + Bevy(ECS 기반 2D)
  - 10 Hz 고정 틱(Decisions) + 60 fps 렌더 보간, 안전한 메모리 모델.
- **UI/대시보드**: Bevy + bevy_egui
  - 전술/페르소나 슬라이더, 로그/해설 패널, 하이라이트 갤러리.
- **LLM(NLG)**: Ollama/llama.cpp 로컬 모델 + `reqwest`(SSE) + `tokio`
  - 권장: Qwen2.5 7B Instruct(Q4) 또는 Llama 3.1 8B Instruct(Q4).
  - 톤 제어: 시스템 프롬프트 스위치(침착/공격적/흥분), 스트리밍 우선.
- **TTS(선택)**: Piper/Coqui TTS(한국어) — HUD 온/오프.
- **이미지(하이라이트 스틸)**: SDXL-Turbo/SD1.5(ComfyUI/Invoke HTTP 호출)
  - 속도/품질 모드, 큐/캐시 비동기 처리.
- **스토리지**: `rusqlite`(SQLite), 파일 시스템(이미지 캐시)
- **로깅/성능**: `tracing`, `tracing-subscriber`
- **결정론/RNG**: `rand_chacha` 시드 고정(경기 단위 재현성)

## 4. 아키텍처 개요
- **sim-core(crate)**: 헤드리스 경기 엔진(ECS 컴포넌트/시스템, 물리/의사결정/이벤트 발생). 렌더 의존성 없음.
- **app-desktop(Bevy)**: 렌더링/입력/UI 통합. 플러그인 집합 구동 및 상태 시각화.
- **nlg-plugin**: 이벤트 버퍼→로컬 LLM(HTTP SSE)→문장 스트림. 1s 초과 시 템플릿 백오프.
- **highlight-plugin**: 중요 이벤트 큐→이미지 엔진 HTTP→아티팩트 저장→갤러리 갱신(캐시/시드).
- **storage-layer**: SQLite 로그/설정, 파일 시스템 이미지 캐시.
- **ffi-capi(선택/후속)**: `cdylib`로 C ABI 내보내기(향후 Unreal 연동 경로).
- **프로세스 모델**: 단일 데스크톱 프로세스 내 비동기 태스크로 NLG/하이라이트 처리, 이미지 엔진은 로컬 HTTP.

## 5. 데이터 스키마(초안)
- **Player**: id, teamId, role, personaParams, position(x,y), stamina, morale, hasBall
- **PersonaParams(예)**
  - riskAppetite(0~1), pressingIntensity(0~1), visionRange(m), patience(0~1), workRate(0~1)
  - patternPreference: {switchPlay:0~1, throughBall:0~1, cutBack:0~1}
  - discipline(0~1), aggression(0~1), confidence(0~1)
- **Tactics**: formation(5v5: 2-1-1, 3-1-0 등), roles(zone, behavior), triggers(pressOnLoss, overlapLeft)
- **Event**: id, t(ms), period(H1/H2), type, teamId, playerId, payload, location, outcome
  - type 예: Pass, Shot, Tackle, Interception, Press, Turnover, SetPiece, Foul, Save, Goal
- **Commentary**: eventId, text, tone, latencyMs
- **Highlight**: eventId, status(queued|rendering|done|error), prompt, imagePath, durationMs

예시(Event)
```json
{
  "id": "e_123",
  "t": 42150,
  "period": "H1",
  "type": "Pass",
  "teamId": "home",
  "playerId": "p_10",
  "location": {"x": 32.5, "y": 41.2},
  "payload": {"targetPlayerId": "p_9", "distance": 12.3, "risk": 0.62},
  "outcome": "Complete"
}
```

## 6. 의사결정 루프(개요)
- 주기: 10 Hz. 각 틱에서 전 플레이어에 대하여 후보 행동을 평가하고 최고 유틸리티 행동 선택.
- 후보 행동: 유지(드리블/보존), 패스(안전/리스크), 슈팅, 압박, 위치 복귀, 커버.
- 간이 유틸리티 점수 예시
  - passUtility = f(시야 내 동료 가시성, 위험 선호, 차단 위험, 전진 이득)
  - shootUtility = f(거리, 각도, 자신감, 수비 압박)
  - pressUtility = f(가까운 상대와의 거리, pressingIntensity, 체력, 전술 트리거)
- 물리/이동: 간단한 가속/감속, 충돌 회피(최소 거리 제약), 공-플레이어 소유권 규칙.

## 7. 이벤트→NLG(로컬 LLM)
- 입력: 최근 N개 이벤트 윈도우 + 현재 이벤트 + 팀/선수 컨텍스트 + 톤.
- 출력: 1~2문장 내의 방송형 문장. 예: "왼쪽에서 빠른 원투, 위험한 스루패스!"
- 프롬프트 구성(요약)
  - system: 역할(중립 해설자), 톤(침착|공격적|흥분).
  - user: 이벤트 요약(JSON), 컨텍스트(팀/선수명, 페르소나 특성 키워드), 글자수/시간 제한.
- 백오프: 템플릿(NLG 룰)로 대체, LLM 응답 지연 > 1s일 경우 즉시 대체 후, LLM 결과 도착 시 교체 가능.

## 8. 하이라이트 스틸(비동기)
- 트리거: Goal, BigSave, Woodwork, Last-ditch Tackle, Long-shot 등.
- 파이프라인: 이벤트→프롬프트 템플릿→큐→로컬 이미지 엔진 호출→파일 저장→갤러리 갱신.
- 성능 목표: 큐 대기 포함 10초 내 1024px 스틸 1장. 동일 이벤트 재요청 시 캐시 반환.

## 9. 대시보드(기능)
- 경기 제어: 시작/일시정지/리셋/속도(0.5x/1x/2x).
- 전술/페르소나: 포메이션 선택, 역할 존 조정, 페르소나 슬라이더(bevy_egui 패널).
- 시각화: Bevy 2D 렌더(공/플레이어/패스/슈팅 트레일), 프레임/틱 시간 표시.
- 로그: 이벤트 타임라인, 실시간 해설(스트리밍), 하이라이트 갤러리.

## 10. 성능/품질 목표
- 시뮬: 10 Hz 의사결정(> 100 match tps 가능), UI 60 fps.
- NLG: 평균 < 500ms/문장(스트리밍 우선), 백오프 즉시.
- 이미지: 스틸 1장 < 10s(캐시 적중 시 < 1s).
- 체감: 페르소나/전술 변경 시 2분 내 명확한 플레이 차이 관찰.

## 11. 위험과 완화
- 로컬 LLM 한국어 자연도 변동: Qwen2.5 7B Instruct 우선 시험, 결과 미흡 시 Llama3.1 8B와 비교.
- 이미지 품질/일관성: 카메라/구도/유니폼 토큰 템플릿화, LoRA 도입은 Phase 1 고려.
- 다중 에이전트 불안정: 초기에는 명시적 전술 제약/냉각계수로 폭주 방지.
- 성능 이슈: 시뮬 계산은 워커 분리, NLG/이미지는 비동기 큐 처리, 캐시 적극 사용.

## 12. 일정(권장)
- 주 1: 시뮬 스켈레톤(엔티티/이동/공 소유권), 이벤트 로그, 캔버스 렌더
- 주 2: 페르소나 파라미터/유틸리티 의사결정, 5v5 규칙 안정화
- 주 3: 전술 캔버스 초안(포메이션/역할), 이벤트 스키마 고정
- 주 4: NLG(로컬 LLM) 통합, 톤 제어/백오프 템플릿
- 주 5: 하이라이트 스틸 파이프라인(큐/캐시/갤러리)
- 주 6: 텔레메트리/리플레이 저장, 품질/성능 튜닝 → 수직 슬라이스 데모

## 13. 수락 기준(모듈별)
- 시뮬: 10분 경기 무충돌/무에러 진행, 평균 프레임 시간 로그 보고.
- 페르소나: 위험 선호/압박 강도 변화가 패스 선택/라인 높이에 정량 반영.
- NLG: 20개 이벤트 템플릿 케이스 커버, 로컬 LLM/백오프 양 경로 테스트 통과.
- 이미지: 5개 트리거 이벤트에 대해 스틸 자동 생성 및 갤러리 노출.
- 대시보드: 전술/페르소나 조정이 실시간 반영, 로그/해설/갤러리 정상 동작.

## 14. 개발 환경
- Rust 툴체인(rustup) + Xcode Command Line Tools(macOS), `cargo-watch`(선택)
- Ollama/llama.cpp 로컬 설치 및 모델(Q4) 프리캐시.
- ComfyUI 또는 Invoke 중 택1, HTTP 엔드포인트 포트 고정.
- Cargo 워크스페이스 권장 구조:
  - `crates/sim-core`(lib) — 헤드리스 엔진
  - `crates/app-desktop`(bin) — Bevy 앱
  - `crates/nlg-plugin`(lib) — LLM 연동
  - `crates/highlight-plugin`(lib) — 이미지 큐/갤러리
  - `crates/storage`(lib) — rusqlite 래퍼
  - `crates/ffi-capi`(lib, 선택) — C ABI 내보내기
- 실행:
  - `cargo run -p app-desktop`
  - 개발: `cargo watch -x 'run -p app-desktop'`

## 15. 다음 행동 항목(바로 실행 가능)
- 페르소나 파라미터 사양 1p 고정: risk/press/vision/patience/workRate/discipline/aggression/confidence.
- 이벤트 스키마 JSON 정의 및 샘플 20개 생성.
- Cargo 워크스페이스 생성 및 크레이트 스켈레톤 구성(sim-core, app-desktop 등).
- FixedTime(100ms) 결정 루프 + 기본 이동/충돌/소유권 규칙 구현.
- bevy_egui 패널: 페르소나/전술 슬라이더 연결, 이벤트 로그 뷰어.
- NLG 프롬프트 템플릿 20개 및 톤 플래그, Ollama SSE 연동 확인.
- 하이라이트 큐/캐시 골격 및 ComfyUI/Invoke HTTP 연동 테스트.

## 16. 로컬 LLM로 시작하는 것에 대한 의견
- **찬성**: 비용/지연/프라이버시 측면에서 빠른 실험에 유리, 템플릿 백오프로 안정성 확보 가능.
- **권장 모델**: Qwen2.5 7B Instruct(Q4) 또는 Llama 3.1 8B Instruct(Q4). 한국어 톤/문체 테스트 후 선택.
- **주의점**: 로컬 성능 하락 시 문장 길이/온도/탑P 제한, 이벤트 축약 요약으로 토큰 절감. 필요 시 선택적 클라우드 백업 경로 유지.


## 17. 최초 환경 설정(macOS 예시)
- Xcode CLT 설치: `xcode-select --install`
- Rust 설치: `curl https://sh.rustup.rs -sSf | sh` 후 `source "$HOME/.cargo/env"`
- 워크스페이스 생성
  - 루트에 `Cargo.toml` 생성 후 아래 내용 추가
```toml
[workspace]
members = [
  "crates/app-desktop",
  "crates/sim-core",
  "crates/nlg-plugin",
  "crates/highlight-plugin",
  "crates/storage"
]
resolver = "2"
```
  - 디렉토리/크레이트 생성
    - `mkdir -p crates && cd crates`
    - `cargo new app-desktop --bin`
    - `cargo new sim-core --lib`
    - `cargo new nlg-plugin --lib`
    - `cargo new highlight-plugin --lib`
    - `cargo new storage --lib`
- 공통 의존성(각 crate `Cargo.toml`에 필요 시 추가)
  - `bevy`, `bevy_egui`, `bevy_prototype_lyon`, `serde`, `serde_json`, `tokio`, `reqwest`, `rusqlite`, `tracing`, `tracing-subscriber`, `rand_chacha`
- Ollama 설치: `brew install ollama` → `ollama serve &` → `ollama pull qwen2.5:7b-instruct-q4_0`
- 이미지 엔진: ComfyUI(권장) 포트 8188 고정, REST 워크플로우 프리셋 준비
- 환경변수(.env 예시)
```env
OLLAMA_BASE_URL=http://127.0.0.1:11434
NLG_MODEL=qwen2.5:7b-instruct-q4_0
VISION_BASE_URL=http://127.0.0.1:8188
```
- 실행 명령
  - `cargo run -p app-desktop`
  - 개발 편의: `cargo install cargo-watch` 후 `cargo watch -x 'run -p app-desktop'`

## 18. 구현 시작(스켈레톤)
- `sim-core` 핵심 타입(예)
```rust
pub struct Player { pub id: u32, pub team: u8, pub stamina: f32, pub morale: f32 }
pub struct Ball { pub owner: Option<u32> }
pub struct Persona { pub risk: f32, pub press: f32, pub vision: f32, pub patience: f32, pub work_rate: f32 }
pub enum MatchEvent { Pass{from:u32,to:u32}, Shot{by:u32}, Tackle{by:u32, on:u32}, Goal{by:u32} }
```
- `app-desktop` 엔트리(요약)
```rust
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

fn main() {
  App::new()
    .insert_resource(Time::<Fixed>::from_hz(10.0))
    .add_plugins((DefaultPlugins, EguiPlugin))
    .add_systems(Startup, setup)
    .add_systems(FixedUpdate, (decide_actions, advance_ball))
    .add_systems(Update, (render_hud,))
    .run();
}

fn setup(mut commands: Commands) { /* 월드/엔티티 배치 */ }
fn decide_actions() { /* 페르소나/전술→유틸리티 선택 */ }
fn advance_ball() { /* 이동/충돌/소유권 */ }
fn render_hud() { /* egui 슬라이더/로그 */ }
```
- NLG 연동(개요): 이벤트 링버퍼→`reqwest` SSE→문장 스트림→HUD 표시, 1s 초과 시 템플릿 출력
- 하이라이트: 트리거 이벤트→프롬프트 템플릿→HTTP 호출→이미지 저장→갤러리 목록 갱신

