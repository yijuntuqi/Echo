//! Evolution: the pet's stage, and the personality that drifts as you interact.
//!
//! State lives in the database as an append-only log of transitions, so the
//! current stage can always be recomputed by replaying it. That keeps upgrades
//! safe: a schema change never has to migrate a denormalised snapshot.

use serde::{Deserialize, Serialize};
use tauri::Emitter;

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

impl Trigger {
    /// Stored in the `trigger_type` column; snake_case to match the serde form.
    pub fn as_str(self) -> &'static str {
        match self {
            Trigger::MoodPositive => "mood_positive",
            Trigger::MoodNegative => "mood_negative",
            Trigger::SpecialEvent => "special_event",
            Trigger::TimeElapsed => "time_elapsed",
            Trigger::InteractionMilestone => "interaction_milestone",
            Trigger::MemoryMilestone => "memory_milestone",
            Trigger::Manual => "manual",
        }
    }
}

/// The live signals [`EvolutionEngine::compute_score`] weighs. One snapshot of
/// what the pet has experienced lately, gathered with three small queries.
///
/// Mood weight is intensity (0..1), not valence — an intense bad day still
/// counts as engagement for now; separating valence is future work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Signals {
    pub avg_mood: f32,
    /// Chats across the trailing week, not today: score is recomputed from
    /// scratch on every evaluation, so a per-day count would never accumulate.
    pub interactions_week: u32,
    pub important_memories: u32,
    pub days_installed: u32,
}

/// How much each signal moves the score. Tuned so roughly a week of steady
/// chatting plus a couple of memories covers the first stage, and each later
/// stage costs twice as much again.
#[derive(Debug, Clone, Copy)]
pub struct EvolutionWeights {
    pub mood: f32,
    pub interaction: f32,
    pub memory: f32,
    pub time: f32,
    pub event: f32,
    /// Score needed at the first stage; later stages cost twice each.
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

    /// Score a transition from the signals available at this moment, as a
    /// fraction of what the current stage costs: `1.0` means "evolve now".
    ///
    /// Pure so it can be unit tested without a database.
    pub fn compute_score(&self, stage: Stage, signals: Signals, trigger: Trigger) -> f32 {
        let event_bonus = if trigger == Trigger::SpecialEvent { 50.0 } else { 0.0 };
        let raw = self.weights.mood * signals.avg_mood * 10.0
            + self.weights.interaction * signals.interactions_week as f32 * 3.0
            + self.weights.memory * signals.important_memories as f32 * 5.0
            + self.weights.time * signals.days_installed as f32 * 0.5
            + self.weights.event * event_bonus;
        // Each stage costs twice the previous one (see `threshold`).
        raw / self.threshold(stage)
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
    let state = crate::state();
    let engine = &state.evolution;
    let pool = state.db.get();
    let mut current = engine.load_state(pool).await;
    // Fill in a live progress figure while we are here: the dashboard's ring
    // would otherwise always read 0 between transitions. Same cheap query set
    // as `evaluate`, no model round trip.
    if let Some(pool) = pool {
        if let Ok(signals) = gather_signals(pool).await {
            let score = engine.compute_score(current.stage, signals, Trigger::TimeElapsed);
            current.progress = score.min(1.0);
        }
    }
    Ok(current)
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

/// Pull the live signals out of the database. Local dates throughout: the
/// mood keys and `install_date` are written with chrono's local clock, while
/// conversation timestamps default to UTC, so both sides get 'localtime'.
pub(crate) async fn gather_signals(pool: &DbPool) -> Result<Signals, sqlx::Error> {
    let avg: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(weight) FROM moods
         WHERE date >= date('now', 'localtime', '-6 days')",
    )
    .fetch_one(pool)
    .await?;

    let interactions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM conversations
         WHERE date(timestamp, 'localtime') >= date('now', 'localtime', '-6 days')",
    )
    .fetch_one(pool)
    .await?;

    let memories: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE importance >= 2")
            .fetch_one(pool)
            .await?;

    // fetch_optional, not fetch_one: an empty profiles table is the normal
    // pre-onboarding state, and `Option` in the type only absorbs a NULL
    // value, not a missing row.
    let days: Option<f64> = sqlx::query_scalar(
        "SELECT julianday(date('now', 'localtime')) - julianday(install_date)
         FROM profiles WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(Signals {
        avg_mood: avg.unwrap_or(0.0).max(0.0) as f32,
        interactions_week: interactions.max(0) as u32,
        important_memories: memories.max(0) as u32,
        days_installed: days.unwrap_or(0.0).max(0.0) as u32,
    })
}

/// Payload of the `evolution:triggered` event (mirrored by `events.ts`).
/// `personality` carries the f32 dimensions — the record's `BLOB` stays
/// encoded in the database, never on the wire.
#[derive(Debug, Clone, Serialize)]
pub struct Evolved {
    pub from_stage: Stage,
    pub to_stage: Stage,
    pub trigger: &'static str,
    pub personality: Vec<f32>,
    pub score: f32,
}

/// Score the pet against its current stage; on a clearing score write the
/// transition row and push `evolution:triggered`. Returns the transition when
/// one happened.
///
/// Cheap by contract (three small queries, no LLM) so callers may run it after
/// every chat turn as well as from the daily scheduler job. A missing database
/// or an already-ultimate pet is a quiet no-op, and failures are logged rather
/// than surfaced — growth is never worth an error toast.
pub async fn evaluate(app: &tauri::AppHandle, trigger: Trigger) -> Option<Evolved> {
    let Ok(pool) = crate::pool() else { return None };
    let evolved = evaluate_with_pool(pool, trigger).await;
    if let Some(evolved) = &evolved {
        if let Err(e) = app.emit("evolution:triggered", evolved) {
            tracing::warn!("evolution event failed: {e}");
        }
    }
    evolved
}

/// [`evaluate`] without the event side effect, so tests can drive the whole
/// state machine against a temporary database.
pub(crate) async fn evaluate_with_pool(pool: &DbPool, trigger: Trigger) -> Option<Evolved> {
    let engine = &crate::state().evolution;
    let state = engine.load_state(Some(pool)).await;
    // Ultimate is final; nothing left to grow into.
    let Some(next) = state.stage.next() else { return None };

    let Ok(signals) = gather_signals(pool).await else { return None };
    let score = engine.compute_score(state.stage, signals, trigger);
    if score < 1.0 {
        tracing::debug!(stage = state.stage.as_str(), score, "not evolving yet");
        return None;
    }

    let result = sqlx::query(
        "INSERT INTO evolution_events (from_stage, to_stage, trigger_type, score, personality_vector)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(state.stage.as_str())
    .bind(next.as_str())
    .bind(trigger.as_str())
    .bind(score as f64)
    .bind(encode_personality(&state.personality))
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!(
                from = state.stage.as_str(),
                to = next.as_str(),
                trigger = trigger.as_str(),
                score,
                "pet evolved"
            );
            Some(Evolved {
                from_stage: state.stage,
                to_stage: next,
                trigger: trigger.as_str(),
                personality: state.personality,
                score,
            })
        }
        Err(e) => {
            tracing::warn!("evolution insert failed: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_is_a_fraction_of_the_stage_cost() {
        let engine = EvolutionEngine::new();
        let signals = Signals {
            avg_mood: 0.8,
            interactions_week: 0,
            important_memories: 0,
            days_installed: 0,
        };

        // Mood alone: 0.30 * 0.8 * 10 = 2.4 of the egg's 100 → 2.4%.
        let score = engine.compute_score(Stage::Egg, signals, Trigger::TimeElapsed);
        assert!((score - 0.024).abs() < 1e-4, "egg score {score}");

        // The same signals count half at the next stage: each stage costs
        // twice the previous one.
        let score = engine.compute_score(Stage::Child, signals, Trigger::TimeElapsed);
        assert!((score - 0.012).abs() < 1e-4, "child score {score}");
    }

    #[test]
    fn special_events_pay_a_bonus() {
        let engine = EvolutionEngine::new();
        let signals = Signals {
            avg_mood: 0.0,
            interactions_week: 0,
            important_memories: 0,
            days_installed: 0,
        };
        let plain = engine.compute_score(Stage::Egg, signals, Trigger::TimeElapsed);
        let boosted = engine.compute_score(Stage::Egg, signals, Trigger::SpecialEvent);
        assert!((boosted - plain - 0.075).abs() < 1e-4, "bonus {boosted}");
    }

    #[tokio::test]
    async fn signals_obey_their_windows() {
        let dir = std::env::temp_dir().join(format!("echo-evo-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let manager = crate::db::DbManager::open_at(dir.join("t.db"), "pw").await.unwrap();
        let pool = manager.pool();

        // A mood from last week and one from today: only the recent one counts.
        let today = chrono::Local::now().date_naive().to_string();
        let week_ago = (chrono::Local::now().date_naive() - chrono::Duration::days(8)).to_string();
        for (date, weight) in [(&week_ago, 0.9), (&today, 0.5)] {
            sqlx::query("INSERT INTO moods (date, emotion, weight, source) VALUES (?, 'happy', ?, 'auto')")
                .bind(date)
                .bind(weight)
                .execute(pool)
                .await
                .unwrap();
        }

        // Two conversations this week (default UTC timestamp) and one from
        // long ago.
        for days_ago in [0i64, 2, 40] {
            sqlx::query(
                "INSERT INTO conversations (timestamp, user_message, ai_reply)
                 VALUES (datetime('now', ?), 'hi', 'ho')",
            )
            .bind(format!("-{days_ago} days"))
            .execute(pool)
            .await
            .unwrap();
        }

        // One important memory, one trivial one.
        sqlx::query("INSERT INTO events (date, description, type, importance) VALUES (?, '毕业', 'milestone', 3)")
            .bind(&today)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO events (date, description, type, importance) VALUES (?, '琐碎', 'note', 1)")
            .bind(&today)
            .execute(pool)
            .await
            .unwrap();

        // No profile row yet: days_installed must read 0 rather than NULL.
        let signals = gather_signals(pool).await.unwrap();
        assert!((signals.avg_mood - 0.5).abs() < 1e-5, "only this week's mood: {signals:?}");
        assert_eq!(signals.interactions_week, 2, "only this week's chats");
        assert_eq!(signals.important_memories, 1, "only important events");
        assert_eq!(signals.days_installed, 0, "no profile row yet");

        // With a profile installed a week ago, the day count appears.
        sqlx::query("INSERT INTO profiles (id, nickname, birthday, install_date) VALUES (1, '测试', '2000-01-01', ?)")
            .bind(&week_ago)
            .execute(pool)
            .await
            .unwrap();
        let signals = gather_signals(pool).await.unwrap();
        assert_eq!(signals.days_installed, 8, "a week and a day after install");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn evaluate_grows_once_then_waits() {
        let dir = std::env::temp_dir().join(format!("echo-evo-run-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let manager = crate::db::DbManager::open_at(dir.join("t.db"), "pw").await.unwrap();
        let pool = manager.pool();

        // A busy first week: 150 chats ≈ 150 * 0.6 = 90, plus 30 important
        // memories at 1.0 each = 30 — together past the egg's 100.
        for i in 0..150 {
            sqlx::query(
                "INSERT INTO conversations (timestamp, user_message, ai_reply)
                 VALUES (datetime('now', '-1 day'), ?, 'ho')",
            )
            .bind(format!("msg{i}"))
            .execute(pool)
            .await
            .unwrap();
        }
        for i in 0..30 {
            sqlx::query("INSERT INTO events (date, description, type, importance) VALUES (date('now','localtime'), ?, 'milestone', 3)")
                .bind(format!("大事{i}"))
                .execute(pool)
                .await
                .unwrap();
        }

        // First evaluation clears the egg's threshold.
        let evolved = evaluate_with_pool(pool, Trigger::InteractionMilestone).await;
        let evolved = evolved.expect("signals clear the egg threshold");
        assert_eq!(evolved.from_stage, Stage::Egg);
        assert_eq!(evolved.to_stage, Stage::Child);
        assert_eq!(evolved.trigger, "interaction_milestone");
        assert_eq!(evolved.personality.len(), 32);

        // The transition is in the log…
        let (from, to): (String, String) = sqlx::query_as(
            "SELECT from_stage, to_stage FROM evolution_events ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!((from.as_str(), to.as_str()), ("egg", "child"));

        // …and the second evaluation starts from child, whose threshold is
        // twice as high: the same signals now hold.
        let second = evaluate_with_pool(pool, Trigger::InteractionMilestone).await;
        assert!(second.is_none(), "child must not clear on the same day");

        let state = crate::state().evolution.load_state(Some(pool)).await;
        assert_eq!(state.stage, Stage::Child);

        std::fs::remove_dir_all(&dir).ok();
    }
}
