//! Emotion analyzer: dual-track (rule + LLM)

use crate::chat::ChatEngine;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Emotion {
    Happy, Sad, Anxious, Calm, Angry, Excited, Bored, Lonely, Grateful, Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionResult {
    pub emotion: Emotion,
    pub confidence: f32,
    pub source: Source,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Source { Rule, LLM, Hybrid }

pub struct RuleEngine;

impl RuleEngine {
    pub fn analyze(text: &str) -> EmotionResult {
        // TODO: 实现关键词匹配
        EmotionResult { emotion: Emotion::Neutral, confidence: 0.3, source: Source::Rule }
    }
}

pub struct LlmEmotionAnalyzer {
    chat: ChatEngine,
}

impl LlmEmotionAnalyzer {
    pub fn new(chat: ChatEngine) -> Self { Self { chat } }
    
    pub async fn analyze(&self, _user_msg: &str, _ai_reply: &str) -> Result<EmotionResult, String> {
        Ok(EmotionResult { emotion: Emotion::Neutral, confidence: 0.5, source: Source::LLM })
    }
}

pub struct HybridAnalyzer {
    rule: RuleEngine,
    llm: LlmEmotionAnalyzer,
}

impl HybridAnalyzer {
    pub fn new(chat: ChatEngine) -> Self {
        Self { rule: RuleEngine, llm: LlmEmotionAnalyzer::new(chat) }
    }
    
    pub async fn analyze(&self, user_msg: &str, ai_reply: &str) -> EmotionResult {
        let rule_res = Self::rule_analyze(user_msg);
        let llm_res = self.llm.analyze(user_msg, ai_reply).await.unwrap_or_else(|_| rule_res.clone());
        
        // 简单融合：取置信度高的
        if rule_res.confidence > llm_res.confidence { rule_res } else { llm_res }
    }
    
    fn rule_analyze(text: &str) -> EmotionResult {
        RuleEngine::analyze(text)
    }
}