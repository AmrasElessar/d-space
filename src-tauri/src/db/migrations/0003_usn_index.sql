-- SPDX-License-Identifier: GPL-3.0-or-later
-- D-Space migration 0003 — Sprint 3.8 USN Journal Index.
--
-- Bölüm referansları: Discovery Log #005 (Bölüm 5.6 — Everything modeli).
--
-- NTFS USN Journal üzerinden persistent file index. Açılışta < 200 ms
-- load için tüm cilt için tek bir tablo, (volume_id, file_ref) PK ile
-- O(1) erişim. Substring arama için name üzerinde ayrı bir non-unique
-- index. Parent_ref ile recursive path zinciri çözülür.
--
-- `usn_watermark` tablosu her cilt için son işlenen USN + journal_id.
-- Journal_id değiştiyse caller wraparound algılar ve full re-enumerate
-- yapar (USN_REASON_* tarihsel kayıtlar kaybolmuş demektir).

CREATE TABLE usn_index (
    volume_id      TEXT    NOT NULL,
    file_ref       INTEGER NOT NULL,
    parent_ref     INTEGER NOT NULL,
    name           TEXT    NOT NULL,
    usn_id         INTEGER NOT NULL,
    last_seen_unix INTEGER NOT NULL,
    attrs          INTEGER NOT NULL,
    PRIMARY KEY (volume_id, file_ref)
);

CREATE INDEX idx_usn_name   ON usn_index(name);
CREATE INDEX idx_usn_parent ON usn_index(volume_id, parent_ref);

CREATE TABLE usn_watermark (
    volume_id  TEXT    PRIMARY KEY,
    next_usn   INTEGER NOT NULL,
    journal_id INTEGER NOT NULL
);
