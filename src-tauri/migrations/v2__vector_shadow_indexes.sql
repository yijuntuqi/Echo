-- v2: sqlite-vec shadow table indexes for performance
-- sqlite-vec 0.2+ creates shadow tables automatically
-- Add indexes on shadow tables for time-range filtering

CREATE INDEX IF NOT EXISTS idx_vec_memories_created_at ON vec_memories_shadow(created_at);
CREATE INDEX IF NOT EXISTS idx_vec_memories_type ON vec_memories_shadow(memory_type);