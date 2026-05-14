-- SPDX-License-Identifier: GPL-3.0-or-later
-- D-Space initial schema — Master mimari Bölüm 14, 8.5, 12.5, 6.5.2.
-- Migration: forward-only (rusqlite_migration).

CREATE TABLE schema_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO schema_meta(key, value) VALUES
    ('app_first_seen', strftime('%s','now')),
    ('spec_version',   'v1.4');

-- Bölüm 4.4 Single Source of Truth: tüm kullanıcı ayarları burada.
CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- ---------------------------------------------------------------
-- Bölüm 8.5 — Snapshot sistemi (Time Machine)
-- ---------------------------------------------------------------
CREATE TABLE snapshots (
    id               INTEGER PRIMARY KEY,
    volume_id        TEXT    NOT NULL,
    captured_at      INTEGER NOT NULL,
    total_size_bytes INTEGER NOT NULL,
    file_count       INTEGER NOT NULL,
    schema_version   INTEGER NOT NULL
);

CREATE INDEX idx_snapshots_volume ON snapshots(volume_id, captured_at DESC);

CREATE TABLE snapshot_entries (
    snapshot_id  INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    path_hash    BLOB    NOT NULL,
    path         TEXT    NOT NULL,
    size_bytes   INTEGER NOT NULL,
    modified_at  INTEGER NOT NULL,
    is_dir       INTEGER NOT NULL,
    PRIMARY KEY (snapshot_id, path_hash)
);

CREATE INDEX idx_snapshot_size
    ON snapshot_entries(snapshot_id, size_bytes DESC);

-- ---------------------------------------------------------------
-- Bölüm 12.5 — Staging + Undo
-- ---------------------------------------------------------------
CREATE TABLE staging_items (
    id             INTEGER PRIMARY KEY,
    original_path  TEXT    NOT NULL,
    staged_path    TEXT    NOT NULL,
    size_bytes     INTEGER NOT NULL,
    staged_at      INTEGER NOT NULL,
    expires_at     INTEGER NOT NULL,
    is_dir         INTEGER NOT NULL,
    reason         TEXT,
    fallback_tier  TEXT  -- 'normal' | 'recycle_bin' | 'cross_volume'
);

CREATE INDEX idx_staging_expires ON staging_items(expires_at);

-- Bölüm 12.3 v1.4 — Cross-volume two-phase commit write-ahead log
CREATE TABLE staging_wal (
    id             INTEGER PRIMARY KEY,
    source_path    TEXT    NOT NULL,
    tmp_path       TEXT,
    target_volume  TEXT    NOT NULL,
    state          TEXT    NOT NULL
                   CHECK (state IN ('BEGIN','COMMITTED','ABORTED')),
    started_at     INTEGER NOT NULL,
    completed_at   INTEGER,
    error_message  TEXT
);

CREATE INDEX idx_wal_state ON staging_wal(state, started_at);

-- Bölüm 12.3.D — Permanent delete forensic trace
CREATE TABLE permanent_deletes_forensic (
    id                    INTEGER PRIMARY KEY,
    original_path         TEXT    NOT NULL,
    size_bytes            INTEGER NOT NULL,
    deleted_at            INTEGER NOT NULL,
    blake3_first4kb       BLOB,
    user_confirmed_twice  INTEGER NOT NULL
);

-- ---------------------------------------------------------------
-- Bölüm 6.5.2 — ML safe-delete cache (v2 stub, schema şimdiden hazır)
-- ---------------------------------------------------------------
CREATE TABLE ml_scores (
    path_hash      BLOB    PRIMARY KEY,
    mtime          INTEGER NOT NULL,
    score          INTEGER NOT NULL CHECK (score BETWEEN 0 AND 100),
    model_version  INTEGER NOT NULL,
    computed_at    INTEGER NOT NULL
);

CREATE INDEX idx_ml_scores_mtime ON ml_scores(mtime);
