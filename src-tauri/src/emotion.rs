//! Emotion detection: a local keyword pass fused with the model's own read.
//!
//! The keyword pass is instant and always available; the model is slower but
//! understands context the keywords miss. Both run, and the more confident
//! result wins.

use serde::{Deserialize, Serialize};

use crate::chat::ChatMessage;

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
    /// Topic words the model pass extracted; empty on rule-only reads.
    #[serde(default)]
    pub topics: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuse_rule_markedly_stronger_wins() {
        let model = EmotionResult {
            emotion: Emotion::Calm,
            confidence: 0.5,
            source: Source::Model,
            topics: vec!["工作".into()],
        };
        let fused = fuse(Some((Emotion::Sad, 0.9)), Some(model)).unwrap();
        assert_eq!(fused.emotion, Emotion::Sad);
        assert_eq!(fused.confidence, 0.9);
        assert_eq!(fused.source, Source::Hybrid);
        // Topics come from the model pass even when the rule read wins.
        assert_eq!(fused.topics, vec!["工作".to_string()]);
    }

    #[test]
    fn fuse_model_wins_when_rule_not_markedly_stronger() {
        let model = EmotionResult {
            emotion: Emotion::Excited,
            confidence: 0.8,
            source: Source::Model,
            topics: vec![],
        };
        let fused = fuse(Some((Emotion::Happy, 0.9)), Some(model)).unwrap();
        assert_eq!(fused.emotion, Emotion::Excited);
        assert_eq!(fused.source, Source::Hybrid);
    }

    #[test]
    fn fuse_model_only_is_hybrid() {
        let model = EmotionResult {
            emotion: Emotion::Lonely,
            confidence: 0.7,
            source: Source::Model,
            topics: vec!["家人".into()],
        };
        let fused = fuse(None, Some(model)).unwrap();
        assert_eq!(fused.emotion, Emotion::Lonely);
        assert_eq!(fused.source, Source::Hybrid);
    }

    #[test]
    fn fuse_rule_only_falls_back_to_rule_source() {
        let fused = fuse(Some((Emotion::Grateful, 1.0)), None).unwrap();
        assert_eq!(fused.emotion, Emotion::Grateful);
        assert_eq!(fused.source, Source::Rule);
        assert!(fused.topics.is_empty());
    }

    #[test]
    fn fuse_both_fail_is_none() {
        assert!(fuse(None, None).is_none());
    }

    #[test]
    fn parse_model_json_accepts_clean_and_fenced() {
        let clean = r#"{"emotion":"happy","weight":0.7,"topics":["考试"]}"#;
        assert_eq!(parse_model_json(clean).unwrap().emotion, Emotion::Happy);

        let fenced = "```json\n{\"emotion\":\"sad\",\"weight\":0.4,\"topics\":[]}\n```";
        let read = parse_model_json(fenced).unwrap();
        assert_eq!(read.emotion, Emotion::Sad);
        assert_eq!(read.topics, Vec::<String>::new());
    }

    #[test]
    fn parse_model_json_rejects_garbage_and_unknown_emotion() {
        assert!(parse_model_json("我觉得还行").is_none());
        assert!(parse_model_json(r#"{"emotion":"ecstatic","weight":0.5}"#).is_none());
    }

    #[test]
    fn parse_model_json_clamps_weight_and_defaults_topics() {
        let read = parse_model_json(r#"{"emotion":"angry","weight":1.7}"#).unwrap();
        assert_eq!(read.confidence, 1.0);
        assert_eq!(read.source, Source::Model);
        assert!(read.topics.is_empty());

        let low = parse_model_json(r#"{"emotion":"calm","weight":-0.3}"#).unwrap();
        assert_eq!(low.confidence, 0.0);
    }
}

impl HybridAnalyzer {
    pub fn new(_chat: &crate::chat::ChatEngine) -> Self {
        Self { _engine: std::marker::PhantomData }
    }

    /// Local-only quick read. Still used by callers that cannot await or
    /// cannot afford a model round trip.
    pub fn analyze(&self, text: &str) -> EmotionResult {
        match RuleEngine::analyze(text) {
            Some((emotion, confidence)) => EmotionResult {
                emotion,
                confidence,
                source: Source::Rule,
                topics: Vec::new(),
            },
            None => EmotionResult {
                emotion: Emotion::Neutral,
                confidence: 0.3,
                source: Source::Rule,
                topics: Vec::new(),
            },
        }
    }

    /// Full read for one chat turn: rule pass plus the model's own read,
    /// fused. `None` means neither pass produced a real read — the caller
    /// should store NULLs rather than a synthetic "neutral".
    ///
    /// The classification target is the USER's emotional state (that is what
    /// the mood diary tracks); the reply rides along as context only.
    pub async fn classify(
        &self,
        chat: &crate::chat::ChatEngine,
        user_text: &str,
        reply_text: &str,
    ) -> Option<EmotionResult> {
        let rule = RuleEngine::analyze(user_text);
        let model = self.model_read(chat, user_text, reply_text).await;
        fuse(rule, model)
    }

    /// Ask the chat engine for a structured emotional read. Any failure —
    /// network, unconfigured key, unparseable answer — is `None`, which the
    /// fusion treats as "no model pass".
    async fn model_read(
        &self,
        chat: &crate::chat::ChatEngine,
        user_text: &str,
        reply_text: &str,
    ) -> Option<EmotionResult> {
        let messages = vec![
            ChatMessage::system(CLASSIFY_PROMPT),
            ChatMessage::user(format!(
                "用户说：{user_text}\nEcho 回复：{reply_text}\n请输出 JSON。"
            )),
        ];
        let outcome = chat.complete(messages, false).await.ok()?;
        parse_model_json(&outcome.reply)
    }
}

/// Instructs the model to answer with exactly one JSON object. Terse on
/// purpose: every token here is paid for on every single chat turn.
const CLASSIFY_PROMPT: &str = "\
你是情绪分析器。只输出一个 JSON 对象，不要输出任何其他文字。\
格式：{\"emotion\":\"happy|sad|anxious|calm|angry|excited|bored|lonely|grateful|neutral\",\
\"weight\":0.0到1.0的小数,\"topics\":[\"主题\",...]}。\
weight 是用户情绪的强度，topics 是这轮对话的主题词（可为空数组）。";

/// Fuse the keyword pass and the model pass.
///
/// The rule read only wins when it is markedly more confident
/// (`rule.confidence >= model.weight + 0.2`); otherwise the model's richer
/// read takes it. `source` is `Hybrid` whenever the model pass ran, and
/// `Rule` only when we fell back to keywords alone.
pub fn fuse(
    rule: Option<(Emotion, f32)>,
    model: Option<EmotionResult>,
) -> Option<EmotionResult> {
    match (rule, model) {
        (Some((emotion, confidence)), Some(model)) => {
            if confidence >= model.confidence + 0.2 {
                Some(EmotionResult {
                    emotion,
                    confidence,
                    source: Source::Hybrid,
                    topics: model.topics,
                })
            } else {
                Some(EmotionResult { source: Source::Hybrid, ..model })
            }
        }
        (Some((emotion, confidence)), None) => Some(EmotionResult {
            emotion,
            confidence,
            source: Source::Rule,
            topics: Vec::new(),
        }),
        (None, Some(mut model)) => {
            model.source = Source::Hybrid;
            Some(model)
        }
        (None, None) => None,
    }
}

/// Parse the model's structured read. Tolerates a ```json fence and any
/// prose around the object: the first `{` to the last `}` is the candidate.
/// Unknown emotion names or unparseable text are `None`; weight is clamped
/// into [0, 1].
fn parse_model_json(raw: &str) -> Option<EmotionResult> {
    #[derive(serde::Deserialize)]
    struct ModelRead {
        emotion: Emotion,
        #[serde(default = "default_weight")]
        weight: f32,
        #[serde(default)]
        topics: Vec<String>,
    }
    fn default_weight() -> f32 {
        0.5
    }

    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end < start {
        return None;
    }
    let read: ModelRead = serde_json::from_str(&raw[start..=end]).ok()?;
    Some(EmotionResult {
        emotion: read.emotion,
        confidence: read.weight.clamp(0.0, 1.0),
        source: Source::Model,
        topics: read.topics,
    })
}
