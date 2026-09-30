//! Evolution: the pet's stage, and the personality that drifts as you interact.
//!
//! State lives in the database as an append-only log of transitions, so the
//! current stage can always be recomputed by replaying it. That keeps upgrades
//! safe: a schema change never has to migrate a denormalised snapshot.

use serde::{Deserialize, Serialize};

use crate::db::DbPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Egg,
    Child,
    Teen,
    Adult,
    Ultimate,
}

impl Stage {
    pub const ORDER: [Stage; 5] = [
        Stage::Egg,
        Stage::Child,
        Stage::Teen,
        Stage::Adult,
        Stage::Ultimate,
    ];

    pub fn index(self) -> usize {
        Self::ORDER.iter().position(|s| *s == self).unwrap_or(0)
    }

    pub fn next(self) -> Option<Stage> {
        Self::ORDER.get(self.index() + 1).copied()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Egg => "egg",
            Stage::Child => "child",
            Stage::Teen => "teen",
            Stage::Adult => "adult",
            Stage::Ultimate => "ultimate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EvolutionRecord {
    pub id: i64,
    pub timestamp: String,
    pub from_stage: String,
    pub to_stage: String,
    pub trigger_type: String,
    pub score: f64,
    pub personality_vector: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionState {
    pub stage: Stage,
    /// 32 dimensions, each in [-1, 1].
    pub personality: Vec<f32>,
    pub history: Vec<EvolutionRecord>,
    /// 0..=1 progress toward the next stage.
    pub progress: f32,
}

/// What pushed the pet toward growing up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    MoodPositive,
    MoodNegative,
    SpecialEvent,
    TimeElapsed,
    InteractionMilestone,
    MemoryMilestone,
    Manual,
}

/// How much each signal moves the score. Tuned so a steady week of chatting is
/// enough to advance, but a single good day is not.
#[derive(Debug, Clone, Copy)]
pub struct EvolutionWeights {
    pub mood: f32,
    pub interaction: f32,
    pub memory: f32,
    pub time: f32,
    pub event: f32,
    /// Score needed at each stage; later stages cost more.
    pub base_threshold: f32,
}

impl Default for EvolutionWeights {
    fn default() -> Self {
        Self {
            mood: 0.30,
            interaction: 0.20,
            memory: 0.20,
            time: 0.15,
            event: 0.15,
            base_threshold: 100.0,
        }
    }
}

pub struct EvolutionEngine {
    weights: EvolutionWeights,
}

impl EvolutionEngine {
    pub fn new() -> Self {
        Self { weights: EvolutionWeights::default() }
    }

    /// Score a transition from the signals available at this moment.
    ///
    /// Pure so it can be unit tested without a database.
    pub fn compute_score(
        &self,
        stage: Stage,
        avg_mood: f32,
        interactions_today: u32,
        important_memories: u32,
        days_installed: u32,
        trigger: Trigger,
    ) -> f32 {
        let event_bonus = if trigger == Trigger::SpecialEvent { 50.0 } else { 0.0 };
        let raw = self.weights.mood * avg_mood * 10.0
            + self.weights.interaction * interactions_today as f32 * 2.0
            + self.weights.memory * important_memories as f32 * 5.0
            + self.weights.time * days_installed as f32 * 0.5
            + self.weights.event * event_bonus;
        // Each stage costs twice the previous one.
        raw / self.weights.base_threshold * 2f32.powi(stage.index() as i32)
    }

    pub fn threshold(&self, stage: Stage) -> f32 {
        self.weights.base_threshold * 2f32.powi(stage.index() as i32)
    }

    /// Current stage, replayed from the transition log. An empty log means a
    /// brand-new pet.
    pub async fn load_state(&self, pool: Option<&DbPool>) -> EvolutionState {
        let Some(pool) = pool else {
            return EvolutionState {
                stage: Stage::Egg,
                personality: vec![0.0; 32],
                history: Vec::new(),
                progress: 0.0,
            };
        };

        let history = sqlx::query_as::<_, EvolutionRecord>(
            "SELECT id, timestamp, from_stage, to_stage, trigger_type, score, personality_vector
             FROM evolution_events ORDER BY id ASC",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let stage = history
            .last()
            .and_then(|r| parse_stage(&r.to_stage))
            .unwrap_or(Stage::Egg);

        let personality = history
            .last()
            .map(|r| decode_personality(&r.personality_vector))
            .unwrap_or_else(|| vec![0.0; 32]);

        EvolutionState { stage, personality, history, progress: 0.0 }
    }
}

pub fn parse_stage(s: &str) -> Option<Stage> {
    match s {
        "egg" => Some(Stage::Egg),
        "child" => Some(Stage::Child),
        "teen" => Some(Stage::Teen),
        "adult" => Some(Stage::Adult),
        "ultimate" => Some(Stage::Ultimate),
        _ => None,
    }
}

fn decode_personality(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

pub fn encode_personality(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[tauri::command]
pub async fn get_evolution_state() -> Result<EvolutionState, String> {
    let engine = &crate::state().evolution;
    let pool = crate::state().db.get();
    Ok(engine.load_state(pool).await)
}

#[tauri::command]
pub async fn force_evolve(stage: Stage) -> Result<(), String> {
    let Some(pool) = crate::state().db.get() else {
        return Err("database not initialised yet".into());
    };
    let current = crate::state().evolution.load_state(Some(pool)).await.stage;

    sqlx::query(
        "INSERT INTO evolution_events (from_stage, to_stage, trigger_type, score, personality_vector)
         VALUES (?, ?, 'manual', 0, ?)",
    )
    .bind(current.as_str())
    .bind(stage.as_str())
    .bind(encode_personality(&vec![0.0f32; 32]))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
