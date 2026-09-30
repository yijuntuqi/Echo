-- Echo v1 Initial Schema
-- SQLite + SQLCipher + sqlite-vec compatible

-- 用户档案（单用户，id 固定为 1）
CREATE TABLE profiles (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    nickname      TEXT NOT NULL,
    birthday      DATE NOT NULL,
    install_date  DATE NOT NULL DEFAULT (date('now')),
    settings_json TEXT NOT NULL DEFAULT '{}',
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 心情日志（每日一条聚合）
CREATE TABLE moods (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    date        DATE NOT NULL UNIQUE,
    emotion     TEXT NOT NULL,
    weight      REAL NOT NULL DEFAULT 1.0,
    note        TEXT,
    source      TEXT NOT NULL DEFAULT 'auto',
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_moods_date ON moods(date);

-- 大事记
CREATE TABLE events (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    date         DATE NOT NULL,
    description  TEXT NOT NULL,
    type         TEXT NOT NULL,
    importance   INTEGER NOT NULL DEFAULT 1,
    tags_json    TEXT DEFAULT '[]',
    created_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_events_date ON events(date);

-- 对话历史
CREATE TABLE conversations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_message    TEXT NOT NULL,
    ai_reply        TEXT NOT NULL,
    emotion         TEXT,
    emotion_weight  REAL,
    topics          TEXT,
    tokens_used     INTEGER,
    model_used      TEXT,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_conv_time ON conversations(timestamp);

-- FTS5 全文搜索
CREATE VIRTUAL TABLE fts_conversations USING fts5(
    user_message, ai_reply,
    content='conversations', content_rowid='id'
);
CREATE TRIGGER conversations_ai AFTER INSERT ON conversations BEGIN
    INSERT INTO fts_conversations(rowid, user_message, ai_reply)
    VALUES (new.id, new.user_message, new.ai_reply);
END;

-- 向量索引（sqlite-vec）
-- 故意不在迁移中创建：vec0 是虚拟表，只有 sqlite-vec 扩展加载成功后才存在。
-- 由 `db::ensure_vector_table` 在运行时按需创建，见 src/db.rs。

-- 进化事件溯源
CREATE TABLE evolution_events (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp             TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    from_stage            TEXT NOT NULL,
    to_stage              TEXT NOT NULL,
    trigger_type          TEXT NOT NULL,
    trigger_ref           INTEGER,
    personality_vector    BLOB,
    score                 REAL,
    metadata_json         TEXT DEFAULT '{}'
);

-- 备份记录
CREATE TABLE backup_logs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    path         TEXT NOT NULL,
    size_bytes   INTEGER NOT NULL,
    status       TEXT NOT NULL,
    error_msg    TEXT,
    created_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);