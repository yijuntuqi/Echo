//! Task 8 — birthday / anniversary greetings (08:00) and the nightly recap
//! (22:00). Copy comes from the premium model via `ChatEngine::complete`;
//! the result lands in `events` (so the timeline and search find it) and in
//! a system notification when the user has notifications enabled.

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::chat::ChatMessage;
use crate::db::parse_date;
use crate::state;

const RECAP_SYSTEM: &str = "你是 Echo，用户的 AI 桌面宠物。请根据今天的对话记录写一段简短的今日回顾：温暖、口语化、中文，80 字以内，不要任何标题或引号，只输出正文。";
const GREET_SYSTEM: &str = "你是 Echo，用户的 AI 桌面宠物。请写一段应景的祝福：温暖真诚、口语化、中文，60 字以内，不要任何标题或引号，只输出正文。";

/// How many of today's turns feed the recap prompt, and how much of each
/// side of a turn survives truncation.
const MAX_TURNS: usize = 20;
const TURN_CLIP: usize = 160;
const NOTICE_CLIP: usize = 120;

/// Which greeting (if any) applies on `today`. The birthday wins when both
/// coincide, and an install date in the current year is not an anniversary
/// yet. `years` is the user's age for birthdays, complete years for
/// anniversaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Greeting {
    pub kind: &'static str, // "birthday" | "anniversary"
    pub years: i32,
}

pub fn greeting_on(
    today: NaiveDate,
    birthday: Option<NaiveDate>,
    install: Option<NaiveDate>,
) -> Option<Greeting> {
    let same_md = |d: NaiveDate| (d.month(), d.day()) == (today.month(), today.day());
    if let Some(bd) = birthday.filter(|d| same_md(*d)) {
        return Some(Greeting { kind: "birthday", years: (today.year() - bd.year()).max(0) });
    }
    if let Some(inst) = install.filter(|d| same_md(*d) && d.year() < today.year()) {
        return Some(Greeting {
            kind: "anniversary",
            years: (today.year() - inst.year()).max(1),
        });
    }
    None
}

/// Collapse newlines and cap the length of one transcript line.
fn clip(s: &str, max: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        let mut out: String = flat.chars().take(max).collect();
        out.push('…');
        out
    }
}

/// Ask the premium model for a short piece of companion copy. Returns `None`
/// when the engine is unconfigured or the reply came back empty — the caller
/// then simply skips this round; no stale content is invented locally.
async fn compose(system: &str, user: String) -> Option<String> {
    let outcome = state()
        .chat
        .complete(
            vec![ChatMessage::system(system), ChatMessage::user(user)],
            true,
        )
        .await
        .ok()?;
    let text = outcome.reply.trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// Store generated copy as a timeline event and notify (respecting the
/// user's notification setting).
async fn publish(
    app: &AppHandle,
    date: String,
    kind: &str,
    importance: i32,
    title: &str,
    text: &str,
) {
    let new = crate::vector::NewEvent {
        date: date.clone(),
        description: text.to_string(),
        kind: kind.to_string(),
        importance,
        tags: vec![kind.to_string()],
    };
    if let Err(e) = crate::vector::add_event(new).await {
        tracing::warn!(error = %e, kind, "recap: failed to store event");
    }

    let allowed = crate::db::get_settings()
        .await
        .map(|s| s.notifications)
        .unwrap_or(true);
    if !allowed {
        return;
    }
    let body: String = text.chars().take(NOTICE_CLIP).collect();
    if let Err(e) = app.notification().builder().title(title).body(&body).show() {
        tracing::warn!(error = %e, "recap: notification failed");
    }
}

/// Local midnight of today as UTC, for timestamp-range queries.
fn day_bounds_utc() -> Option<(chrono::DateTime<Utc>, chrono::DateTime<Utc>)> {
    let now = Local::now();
    let start = Local
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()?
        .with_timezone(&Utc);
    Some((start, start + Duration::days(1)))
}

/// 22:00 — summarise today's conversations into a warm recap card.
pub async fn daily_recap(app: AppHandle) {
    let Ok(pool) = crate::pool() else {
        tracing::debug!("daily recap skipped: database not open yet");
        return;
    };
    let Some((start, end)) = day_bounds_utc() else {
        return;
    };

    let turns: Vec<(String, String)> = match sqlx::query_as(
        "SELECT user_message, ai_reply FROM conversations
         WHERE timestamp >= ? AND timestamp < ? ORDER BY timestamp ASC LIMIT ?",
    )
    .bind(start)
    .bind(end)
    .bind(MAX_TURNS as i64)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "daily recap: conversation query failed");
            return;
        }
    };
    if turns.is_empty() {
        tracing::debug!("daily recap skipped: no conversations today");
        return;
    }

    let mut transcript = String::new();
    for (i, (user, ai)) in turns.iter().enumerate() {
        transcript.push_str(&format!(
            "{}. 用户：{}\n   Echo：{}\n",
            i + 1,
            clip(user, TURN_CLIP),
            clip(ai, TURN_CLIP),
        ));
    }
    let nickname = crate::db::get_settings()
        .await
        .map(|s| s.nickname)
        .unwrap_or_default();

    let prompt = format!(
        "用户昵称：{nickname}\n\n今天的对话记录：\n{transcript}\n请根据以上内容写一段今天的回顾卡片。"
    );
    let Some(text) = compose(RECAP_SYSTEM, prompt).await else {
        tracing::warn!("daily recap: model returned nothing");
        return;
    };

    let date = Local::now().date_naive().format("%Y-%m-%d").to_string();
    publish(&app, date, "recap", 2, "🌙 Echo 的今日回顾", &text).await;
    tracing::info!(chars = text.chars().count(), "daily recap generated");
}

/// 08:00 — birthday and friendship-anniversary greetings.
pub async fn anniversary_check(app: AppHandle) {
    let Ok(pool) = crate::pool() else {
        tracing::debug!("anniversary check skipped: database not open yet");
        return;
    };

    let settings = match crate::db::get_settings().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "anniversary check: settings unavailable");
            return;
        }
    };
    let install: Option<String> = {
        use sqlx::Row;
        sqlx::query("SELECT install_date FROM profiles WHERE id = 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.get::<String, _>("install_date"))
    };

    let today = Local::now().date_naive();
    let Some(greeting) =
        greeting_on(today, parse_date(&settings.birthday), install.as_deref().and_then(parse_date))
    else {
        tracing::debug!("anniversary check: nothing to greet today");
        return;
    };

    let (title, kind, importance, hint) = match greeting.kind {
        "birthday" => (
            format!("🎂 {}，生日快乐！", settings.nickname),
            "birthday",
            4,
            format!("今天是用户的 {} 岁生日。", greeting.years.max(0)),
        ),
        _ => (
            format!("🎉 与 Echo 相识 {} 周年", greeting.years.max(1)),
            "anniversary",
            4,
            format!("今天是用户和 Echo 相识整整 {} 周年的日子。", greeting.years.max(1)),
        ),
    };

    let prompt = format!(
        "用户昵称：{}\n{}\n请写一段应景的祝福。",
        settings.nickname, hint
    );
    let Some(text) = compose(GREET_SYSTEM, prompt).await else {
        tracing::warn!(kind, "anniversary check: model returned nothing");
        return;
    };

    let date = today.format("%Y-%m-%d").to_string();
    publish(&app, date, kind, importance, &title, &text).await;
    tracing::info!(kind, "greeting generated");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn birthday_on_the_day_and_priority_over_anniversary() {
        let today = d(2026, 5, 20);
        let bd = d(2004, 5, 20);
        let inst = d(2024, 5, 20);
        // Birthday wins when both land on the same day.
        assert_eq!(
            greeting_on(today, Some(bd), Some(inst)),
            Some(Greeting { kind: "birthday", years: 22 })
        );
        // Anniversary alone.
        assert_eq!(
            greeting_on(today, None, Some(inst)),
            Some(Greeting { kind: "anniversary", years: 2 })
        );
    }

    #[test]
    fn quiet_days_and_edge_cases() {
        let today = d(2026, 5, 20);
        assert_eq!(greeting_on(today, None, None), None);
        // Different day.
        assert_eq!(greeting_on(today, Some(d(2004, 5, 21)), None), None);
        // Install date in the current year is not an anniversary yet.
        assert_eq!(greeting_on(today, None, Some(d(2026, 5, 20))), None);
        // A birthday on Feb 29 is celebrated on Mar 1 in non-leap years.
        let non_leap = d(2027, 3, 1);
        assert_eq!(
            greeting_on(non_leap, Some(d(2004, 2, 29)), None),
            None,
            "we do not fake leap-day handling; no greeting is safer than a wrong one"
        );
    }

    #[test]
    fn clip_truncates_and_flattens() {
        assert_eq!(clip("a\n b   c", 10), "a b c");
        let long = "x".repeat(300);
        let out = clip(&long, 160);
        assert_eq!(out.chars().count(), 161); // 160 + the ellipsis
        assert!(out.ends_with('…'));
    }
}
