-- SPDX-License-Identifier: GPL-3.0-or-later
-- D-Space migration 0002 — Bölüm 6.4 Kullanıcı Tanımlı Kurallar.
--
-- Built-in rule motoruyla (RULES sabit dizisi, Bölüm 6.2) zincirleme:
-- user rules ÖNCE değerlendirilir; eşleşme yoksa built-in'lere düşer.
-- Pattern tipi `name` (tam isim, case-insensitive) veya `extension`
-- (`.tmp` benzeri). Glob v0.2'de.

CREATE TABLE user_rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern      TEXT    NOT NULL,
    pattern_type TEXT    NOT NULL CHECK (pattern_type IN ('name', 'extension')),
    score        INTEGER NOT NULL CHECK (score BETWEEN 0 AND 100),
    explanation  TEXT    NOT NULL,
    enabled      INTEGER NOT NULL DEFAULT 1,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE INDEX idx_user_rules_enabled ON user_rules(enabled, pattern_type);
