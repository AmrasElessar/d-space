// SPDX-License-Identifier: GPL-3.0-or-later
//
// Tek tipli hata enum'u. Tüm modüller bu Error üzerinden raporlar.
// Tauri komutlarına dönerken serde Serialize uygulanır, frontend
// tarafında typed discriminant ile yakalanır.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO hatası: {0}")]
    Io(#[from] std::io::Error),

    #[error("Veritabanı hatası: {0}")]
    Db(String),

    #[error("Volume hatası: {0}")]
    Volume(String),

    #[error("Tarama hatası: {0}")]
    Scan(String),

    #[error("Staging hatası: {0}")]
    Staging(String),

    #[error("Snapshot hatası: {0}")]
    Snapshot(String),

    #[error("Duplicate detector hatası: {0}")]
    Duplicate(String),

    #[error("Kilitli dosya: {0}")]
    LockedFile(String),

    #[error("USN index hatası: {0}")]
    Index(String),

    #[error("Henüz uygulanmadı: {0}")]
    NotImplemented(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Error", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl Error {
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Io(_) => "io",
            Error::Db(_) => "db",
            Error::Volume(_) => "volume",
            Error::Scan(_) => "scan",
            Error::Staging(_) => "staging",
            Error::Snapshot(_) => "snapshot",
            Error::Duplicate(_) => "duplicate",
            Error::LockedFile(_) => "locked_file",
            Error::Index(_) => "index",
            Error::NotImplemented(_) => "not_implemented",
        }
    }
}
