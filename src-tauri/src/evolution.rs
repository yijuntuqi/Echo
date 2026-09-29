//! Evolution engine: state machine + personality vector + triggers

use serde::{Deserialize, Serialize};
use std::array;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage { Egg, Child, Teen, Adult, Ultimate }

impl Stage {
    pub fn from_days(days: u32, special: bool) -> Self {
        if special { return Stage::Ultimate; }
        match days {
            0..=7 => Stage::Egg,
            8..=30 => Stage::Child,
            31..=90 => Stage::Teen,
            91..=365 => Stage::Adult,
            _ => Stage::Ultimate,
        }
    }
    pub fn next(self) -> Option<Self> {
        match self {
            Stage::Egg => Some(Stage::Child),
            Stage::Child => Some(Stage::Teen),
            Stage::Teen => Some(Stage::Adult),
            Stage::Adult => Some(Stage::Ultimate),
            Stage::Ultimate => None,
        }
    }
    pub fn asset_prefix(&self) -> &'static str {
        match self {
            Stage::Egg => "egg", Stage::Child => "child", Stage::Teen => "teen",
            Stage::Adult => "adult", Stage::Ultimate => "ultimate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityVector(pub [f32; 32]);

impl PersonalityVector {
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self(array::from_fn(|_| rng.gen_range(-1.0..1.0)))
    }
    pub fn zero() -> Self { Self([0.0; 32]) }
    pub fn drift(&mut self, delta: &[f32; 32], strength: f32) {
        for i in 0..32 {
            self.0[i] = (self.0[i] + delta[i] * strength).clamp(-1.0, 1.0);
        }
    }
    pub fn similarity(&self, other: &Self) -> f32 {
        self.0.iter().zip(other.0).map(|(a, b)| a * b).sum()
    }
    pub fn to_bytes(&self) -> Vec<u8> { bincode::serialize(self).unwrap() }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> { bincode::deserialize(bytes) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionWeights {
    pub mood_weight: f32,
    pub interaction_weight: f32,
    pub memory_weight: f32,
    pub time_weight: f32,
    pub event_weight: f32,
    pub threshold_base: f32,
}

impl Default for EvolutionWeights {
    fn default() -> Self {
        Self { mood_weight: 0.3, interaction_weight: 0.2, memory_weight: 0.2,
               time_weight: 0.15, event_weight: 0.15, threshold_base: 100.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    MoodPositive, MoodNegative, SpecialEvent, TimeElapsed,
    InteractionMilestone, MemoryMilestone, Manual,
}

#[derive(Debug, Clone)]
pub struct TriggerContext {
    pub ty: TriggerType,
    pub ref_id: Option<i64>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionResult {
    pub old_stage: Stage,
    pub new_stage: Stage,
    pub personality: PersonalityVector,
    pub score: f32,
    pub trigger: TriggerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub id: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from_stage: Stage,
    pub to_stage: Stage,
    pub trigger_type: String,
    pub trigger_ref: Option<i64>,
    pub personality_vector: Vec<u8>,
    pub score: f32,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvolutionState {
    pub stage: Stage,
    pub personality: Vec<f32>,
    pub history: Vec<EvolutionEvent>,
    pub progress: f32,
}

pub struct EvolutionEngine {
    // db: DbPool,
    // embedding: EmbeddingService,
    personality: PersonalityVector,
    current_stage: Stage,
    weights: EvolutionWeights,
}

impl EvolutionEngine {
    pub async fn new() -> Result<Self, String> {
        Ok(Self {
            personality: PersonalityVector::random(),
            current_stage: Stage::Egg,
            weights: Default::default(),
        })
    }
    
    pub async fn evaluate(&mut self, _trigger: TriggerContext) -> Option<EvolutionResult> {
        None
    }
    
    pub fn get_state(&self) -> EvolutionState {
        EvolutionState {
            stage: self.current_stage,
            personality: self.personality.0.to_vec(),
            history: vec![],
            progress: 0.0,
        }
    }
}

#[tauri::command]
pub async fn get_evolution_state() -> Result<EvolutionState, String> {
    Ok(EvolutionState {
        stage: Stage::Egg,
        personality: vec![0.0; 32],
        history: vec![],
        progress: 0.0,
    })
}

#[tauri::command]
pub async fn force_evolve(stage: Stage) -> Result<(), String> {
    Ok(())
}