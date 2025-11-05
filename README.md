# Eleven FC - Phase 0

AI 네이티브 축구 시뮬레이션 게임의 Phase 0 프로토타입입니다.

## 프로젝트 구조

```
eleven/
├── Cargo.toml          # Workspace 설정
├── crates/
│   ├── app-desktop/     # Bevy 기반 데스크톱 애플리케이션
│   ├── sim-core/        # 헤드리스 경기 엔진 (렌더링 독립)
│   ├── nlg-plugin/      # LLM 해설 플러그인 (예정)
│   ├── highlight-plugin/ # 하이라이트 이미지 생성 플러그인 (예정)
│   └── storage/         # 데이터베이스 레이어 (예정)
└── README.md
```

## 기술 스택

- **시뮬레이션**: Rust + Bevy (ECS 기반)
- **UI**: bevy_egui
- **의사결정**: 페르소나 기반 유틸리티 시스템
- **물리**: 간단한 2D 물리 엔진 (충돌 회피, 이동, 공 소유권)

## 현재 상태

✅ Phase 0 완료:
- Cargo workspace 구조
- sim-core 기본 타입 및 이벤트 시스템
- 기본 물리/이동 시스템
- 유틸리티 기반 의사결정 시스템
- Bevy 앱 스켈레톤 (10Hz FixedTime)
- 기본 대시보드 UI (bevy_egui)
- 5v5 경기 시뮬레이션 및 렌더링

🚧 Phase 1 진행 중:
- decision-plugin 기본 구조 (의도 타입, LLMEngine 인터페이스)
- llama.cpp 통합 준비 중

📋 Phase 1 예정:
- llama.cpp 바인딩 및 Qwen3-8B 모델 로딩
- 프롬프트 생성 및 LLM 호출
- 의도 실행 엔진 (sim-core 통합)
- 전술 캔버스 UI
- 스쿼드 관리

## 실행 방법

```bash
# 빌드 및 실행
cargo run -p app-desktop

# 개발 모드 (파일 변경 시 자동 재빌드)
cargo install cargo-watch  # 처음 한 번만
cargo watch -x 'run -p app-desktop'
```

## 환경 설정

### 필수 도구
- Rust (rustup으로 설치)
- macOS: Xcode Command Line Tools

### 선택 사항 (향후 필요)
- Ollama (로컬 LLM)
- ComfyUI (이미지 생성)

## 개발 가이드

상세한 구현 계획은 [eleven_fc_phase0_plan.md](./eleven_fc_phase0_plan.md)를 참고하세요.
