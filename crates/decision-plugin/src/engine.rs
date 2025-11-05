use crate::context::DecisionContext;
use crate::intent::ActionPlan;
use crate::prompt::PromptGenerator;
use thiserror::Error;
use std::sync::Arc;

/// LLM 엔진 에러
#[derive(Error, Debug)]
pub enum LlmEngineError {
    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    #[error("Timeout")]
    Timeout,
}

impl From<llama_cpp_2::LLamaCppError> for LlmEngineError {
    fn from(err: llama_cpp_2::LLamaCppError) -> Self {
        LlmEngineError::InferenceFailed(format!("llama.cpp error: {}", err))
    }
}

/// LLM 엔진 트레이트
/// 
/// 다양한 LLM 백엔드(직접 llama.cpp, HTTP/Ollama 등)를 추상화
pub trait LlmEngine: Send + Sync {
    /// 의사결정 컨텍스트를 받아서 액션 플랜 생성
    fn generate_action_plan(
        &mut self,
        context: &DecisionContext,
    ) -> Result<ActionPlan, LlmEngineError>;
    
    /// 모델 로딩 상태 확인
    fn is_ready(&self) -> bool;
}

/// Direct llama.cpp 엔진 (Phase 1 구현)
/// 
/// Note: LlamaContext는 lifetime이 있어 Arc로 감쌀 수 없으므로,
/// 필요한 시점에 컨텍스트를 생성하는 방식으로 변경 필요할 수 있음
pub struct DirectLlamaEngine {
    is_loaded: bool,
    model_path: Option<String>,
    backend: Option<Arc<llama_cpp_2::llama_backend::LlamaBackend>>,
    model: Option<Arc<llama_cpp_2::model::LlamaModel>>,
    // Note: LlamaContext는 lifetime이 있어 여기에 직접 저장할 수 없음
    // 대신 필요할 때마다 생성하거나, 다른 방식으로 관리 필요
}

impl DirectLlamaEngine {
    pub fn new() -> Self {
        Self {
            is_loaded: false,
            model_path: None,
            backend: None,
            model: None,
        }
    }
    
    /// 모델 로드 (GGUF 파일 경로)
    pub fn load_model(&mut self, model_path: &str) -> Result<(), LlmEngineError> {
        tracing::info!("Loading model from: {}", model_path);
        
        // 파일 존재 확인
        if !std::path::Path::new(model_path).exists() {
            return Err(LlmEngineError::ModelLoadFailed(
                format!("Model file not found: {}", model_path)
            ));
        }
        
        // Backend 초기화
        let backend = Arc::new(
            llama_cpp_2::llama_backend::LlamaBackend::init()
                .map_err(|e| LlmEngineError::ModelLoadFailed(format!("Backend init failed: {}", e)))?
        );
        
        // 모델 파라미터 설정
        let model_params = llama_cpp_2::model::params::LlamaModelParams::default()
            .with_n_gpu_layers(0); // CPU only for now, can enable Metal later
        
        // 모델 로드
        let model = Arc::new(
            llama_cpp_2::model::LlamaModel::load_from_file(&backend, model_path, &model_params)
                .map_err(|e| LlmEngineError::ModelLoadFailed(format!("Model load failed: {}", e)))?
        );
        
        self.backend = Some(backend);
        self.model = Some(model);
        self.model_path = Some(model_path.to_string());
        self.is_loaded = true;
        
        tracing::info!("Model loaded successfully");
        Ok(())
    }
    
    /// 프롬프트를 LLM에 전달하고 응답 받기
    fn infer(&mut self, prompt: &str) -> Result<String, LlmEngineError> {
        let backend = self.backend.as_ref().ok_or_else(|| 
            LlmEngineError::InferenceFailed("Backend not initialized".to_string())
        )?;
        
        let model = self.model.as_ref().ok_or_else(|| 
            LlmEngineError::InferenceFailed("Model not initialized".to_string())
        )?;
        
        // 컨텍스트 파라미터 설정
        use std::num::NonZeroU32;
        let ctx_params = llama_cpp_2::context::params::LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(4096).unwrap())); // 컨텍스트 크기
        
        // 컨텍스트 생성 (lifetime 때문에 매번 생성해야 할 수 있음)
        let mut context = model.new_context(backend, ctx_params)
            .map_err(|e| LlmEngineError::InferenceFailed(format!("Context creation failed: {}", e)))?;
        
        // 프롬프트를 토큰으로 변환
        let tokens = model.str_to_token(prompt, llama_cpp_2::model::AddBos::Always)
            .map_err(|e| LlmEngineError::InferenceFailed(format!("Tokenization failed: {}", e)))?;
        
        if tokens.is_empty() {
            return Err(LlmEngineError::InferenceFailed("Empty tokens".to_string()));
        }
        
        // 배치 생성 및 초기 토큰 추가
        let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(512, 1);
        for (i, &token) in tokens.iter().enumerate() {
            batch.add(token, i as i32, &[0], false)
                .map_err(|e| LlmEngineError::InferenceFailed(format!("Batch add failed: {}", e)))?;
        }
        
        // 디코드
        context.decode(&mut batch)
            .map_err(|e| LlmEngineError::InferenceFailed(format!("Decode failed: {}", e)))?;
        
        // 생성
        let mut response = String::new();
        let mut n_cur = batch.n_tokens();
        let n_max = 512; // 최대 토큰 수
        
        while n_cur < n_max {
            // 다음 토큰 샘플링 (greedy)
            let candidates = context.candidates_ith(batch.n_tokens() - 1);
            
            let new_token_id = candidates
                .max_by(|a, b| a.logit().partial_cmp(&b.logit()).unwrap_or(std::cmp::Ordering::Equal))
                .map(|c| c.id())
                .ok_or_else(|| LlmEngineError::InferenceFailed("No candidates".to_string()))?;
            
            // EOS 토큰 확인
            if new_token_id == model.token_eos() {
                break;
            }
            
            // 토큰을 문자열로 변환
            let token_str = model.token_to_str(new_token_id, llama_cpp_2::model::Special::Plaintext)
                .map_err(|e| LlmEngineError::InferenceFailed(format!("Token to string failed: {}", e)))?;
            response.push_str(&token_str);
            
            // 배치 비우고 새 토큰 추가
            batch.clear();
            batch.add(new_token_id, n_cur, &[0], true)
                .map_err(|e| LlmEngineError::InferenceFailed(format!("Add token failed: {}", e)))?;
            
            // 디코드
            context.decode(&mut batch)
                .map_err(|e| LlmEngineError::InferenceFailed(format!("Decode failed: {}", e)))?;
            
            n_cur += 1;
        }
        
        Ok(response)
    }
}

impl LlmEngine for DirectLlamaEngine {
    fn generate_action_plan(
        &mut self,
        context: &DecisionContext,
    ) -> Result<ActionPlan, LlmEngineError> {
        if !self.is_loaded {
            return Err(LlmEngineError::ModelLoadFailed("Model not loaded".to_string()));
        }
        
        // 1. 프롬프트 생성
        let prompt = PromptGenerator::generate_prompt(context);
        tracing::debug!("Generated prompt length: {} chars", prompt.len());
        
        // 2. LLM 호출
        let start_time = std::time::Instant::now();
        let response = self.infer(&prompt)?;
        let latency_ms = start_time.elapsed().as_millis() as u64;
        
        tracing::info!("LLM inference completed in {}ms", latency_ms);
        tracing::debug!("LLM response: {}", response);
        
        // 3. JSON 응답 파싱 → ActionPlan
        let action_plan = PromptGenerator::parse_response(
            &response, 
            context.current_time_ms, 
            latency_ms
        ).map_err(|e| LlmEngineError::InvalidResponse(e))?;
        
        Ok(action_plan)
    }
    
    fn is_ready(&self) -> bool {
        self.is_loaded
    }
}
