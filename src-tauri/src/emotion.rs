//! Emotion detection: a local keyword pass fused with the model's own read.
//!
//! The keyword pass is instant and always available; the model is slower but
//! understands context the keywords miss. Both run, and the more confident
//! result wins.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Emotion {
    Happy,
    Sad,
    Anxious,
    Calm,
    Angry,
    Excited,
    Bored,
    Lonely,
    Grateful,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Rule,
    Model,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionResult {
    pub emotion: Emotion,
    /// 0..=1 — how sure we are.
    pub confidence: f32,
    pub source: Source,
}

impl Emotion {
    pub fn as_str(self) -> &'static str {
        match self {
            Emotion::Happy => "happy",
            Emotion::Sad => "sad",
            Emotion::Anxious => "anxious",
            Emotion::Calm => "calm",
            Emotion::Angry => "angry",
            Emotion::Excited => "excited",
            Emotion::Bored => "bored",
            Emotion::Lonely => "lonely",
            Emotion::Grateful => "grateful",
            Emotion::Neutral => "neutral",
        }
    }
}

/// Chinese keywords per emotion with rough intensities. Deliberately small and
/// hand-tuned: it is a fast prior, not a classifier.
const LEXICON: &[(Emotion, &[(&str, f32)])] = &[
    (
        Emotion::Happy,
        &[("开心", 1.0), ("高兴", 0.9), ("快乐", 0.9), ("哈哈", 0.7), ("喜欢", 0.6), ("太棒", 0.8)],
    ),
    (Emotion::Sad, &[("难过", 1.0), ("伤心", 1.0), ("失落", 0.8), ("想哭", 0.9)]),
    (Emotion::Anxious, &[("焦虑", 1.0), ("紧张", 0.8), ("担心", 0.7), ("压力", 0.7)]),
    (Emotion::Calm, &[("平静", 0.9), ("放松", 0.8), ("安心", 0.8), ("还好", 0.5)]),
    (Emotion::Angry, &[("生气", 1.0), ("愤怒", 1.0), ("火大", 0.9), ("气死", 0.9)]),
    (Emotion::Excited, &[("兴奋", 1.0), ("激动", 0.9), ("期待", 0.7)]),
    (Emotion::Bored, &[("无聊", 1.0), ("没意思", 0.8), ("好闲", 0.7)]),
    (Emotion::Lonely, &[("孤独", 1.0), ("寂寞", 0.9), ("一个人", 0.5)]),
    (Emotion::Grateful, &[("谢谢", 0.9), ("感谢", 1.0), ("多谢", 0.7)]),
];

pub struct RuleEngine;

impl RuleEngine {
    /// Score every emotion and return the strongest, or `None` if nothing matched.
    pub fn analyze(text: &str) -> Option<(Emotion, f32)> {
        let mut best: Option<(Emotion, f32)> = None;
        for (emotion, keywords) in LEXICON {
            let score: f32 = keywords
                .iter()
                .filter(|(kw, _)| text.contains(kw))
                .map(|(_, w)| *w)
                .sum();
            if score > 0.0 && best.map_or(true, |(_, b)| score > b) {
                best = Some((*emotion, score));
            }
        }
        // Two or more keyword hits, or one strong hit, is worth acting on.
        best.filter(|(_, s)| *s >= 0.9).map(|(e, s)| (e, (s / 2.0).min(1.0)))
    }
}

pub struct HybridAnalyzer {
    _engine: std::marker::PhantomData<&'static ()>,
}

impl HybridAnalyzer {
    pub fn new(_chat: &crate::chat::ChatEngine) -> Self {
        Self { _engine: std::marker::PhantomData }
    }

    /// Local-only for now. When the model pass lands it is fused in here:
    /// take the model's read unless the keyword pass is markedly more confident.
    pub fn analyze(&self, text: &str) -> EmotionResult {
        match RuleEngine::analyze(text) {
            Some((emotion, confidence)) => EmotionResult {
                emotion,
                confidence,
                source: Source::Rule,
            },
            None => EmotionResult {
                emotion: Emotion::Neutral,
                confidence: 0.3,
                source: Source::Rule,
            },
        }
    }
}
