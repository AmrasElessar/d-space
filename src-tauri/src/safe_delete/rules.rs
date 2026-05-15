// SPDX-License-Identifier: GPL-3.0-or-later
//
// Deklaratif kural motoru — Bölüm 6.1, 6.2 (50+ kural hedefi).
// v0.1 Faz 1: ~30 kural, Name + Extension pattern.
// v0.2'de: path-context (atadan path ipuçları), glob (Bölüm 6.4 user rules).

#[derive(Debug, Clone, Copy)]
pub enum Pattern {
    /// Tam isim eşleşmesi (case-insensitive).
    Name(&'static str),
    /// Dosya uzantısı (`.tmp`, `.log`). Klasörlere uygulanmaz.
    Extension(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub pattern: Pattern,
    /// 0-100. Yüksek = silmek daha güvenli (Bölüm 6.3).
    pub score: u8,
    pub explanation: &'static str,
}

/// Bölüm 6.2 — built-in rule seti. Faz 1: 33 kural.
/// Eşit isimli kural eşleşirse ilk eşleşen kullanılır (sıra önemli).
pub const RULES: &[Rule] = &[
    // ----- Geliştirici cache / artifact -----
    Rule {
        id: "node_modules",
        pattern: Pattern::Name("node_modules"),
        score: 95,
        explanation: "Node.js paket cache'i — `npm install`/`pnpm install` ile yeniden gelir.",
    },
    Rule {
        id: "target",
        pattern: Pattern::Name("target"),
        score: 85,
        explanation: "Rust derleme çıktısı — `cargo build` ile yeniden üretilir.",
    },
    Rule {
        id: "build",
        pattern: Pattern::Name("build"),
        score: 75,
        explanation: "Genel build çıktı klasörü — yeniden üretilebilir.",
    },
    Rule {
        id: "dist",
        pattern: Pattern::Name("dist"),
        score: 80,
        explanation: "Dağıtım build çıktısı — bundler tekrar üretir.",
    },
    Rule {
        id: "out",
        pattern: Pattern::Name("out"),
        score: 75,
        explanation: "Build çıktı klasörü (Next.js, esbuild, vs.) — yeniden üretilir.",
    },
    Rule {
        id: "next-cache",
        pattern: Pattern::Name(".next"),
        score: 90,
        explanation: "Next.js build cache — `next dev`/`build` yeniden oluşturur.",
    },
    Rule {
        id: "nuxt-cache",
        pattern: Pattern::Name(".nuxt"),
        score: 90,
        explanation: "Nuxt build cache — yeniden üretilir.",
    },
    Rule {
        id: "pycache",
        pattern: Pattern::Name("__pycache__"),
        score: 96,
        explanation: "Python bytecode cache — `.py` dosyalarından otomatik üretilir.",
    },
    Rule {
        id: "gradle",
        pattern: Pattern::Name(".gradle"),
        score: 80,
        explanation: "Gradle cache — Gradle yeniden indirir/derler.",
    },
    Rule {
        id: "m2",
        pattern: Pattern::Name(".m2"),
        score: 70,
        explanation: "Maven local repo — silmek tüm projelerin baştan indirmesi anlamına gelir.",
    },
    Rule {
        id: "vscode-extensions",
        pattern: Pattern::Name(".vscode-extensions"),
        score: 60,
        explanation: "VS Code eklenti cache — eklentiler yeniden kurulur.",
    },
    Rule {
        id: "vendor",
        pattern: Pattern::Name("vendor"),
        score: 65,
        explanation: "Composer/Go vendor klasörü — paket yöneticisi yeniden indirir.",
    },
    Rule {
        id: "venv",
        pattern: Pattern::Name("venv"),
        score: 80,
        explanation: "Python virtualenv — `requirements.txt`'ten yeniden kurulur.",
    },
    Rule {
        id: "venv-dot",
        pattern: Pattern::Name(".venv"),
        score: 80,
        explanation: "Python virtualenv (gizli) — `requirements.txt`'ten yeniden kurulur.",
    },
    Rule {
        id: "bower",
        pattern: Pattern::Name("bower_components"),
        score: 92,
        explanation: "Bower (eski) cache — `bower install` ile gelir.",
    },
    // ----- Tarayıcı / oyun launcher cache -----
    Rule {
        id: "shadercache",
        pattern: Pattern::Name("ShaderCache"),
        score: 90,
        explanation: "GPU shader önbelleği — oyun açıldığında yeniden derlenir.",
    },
    Rule {
        id: "crashpad",
        pattern: Pattern::Name("Crashpad"),
        score: 95,
        explanation: "Chromium crash raporu cache — kritik veri değil.",
    },
    Rule {
        id: "code-cache",
        pattern: Pattern::Name("Code Cache"),
        score: 92,
        explanation: "Chromium V8 code cache — tarayıcı yeniden üretir.",
    },
    Rule {
        id: "gpucache",
        pattern: Pattern::Name("GPUCache"),
        score: 92,
        explanation: "Chromium GPU cache — tarayıcı yeniden üretir.",
    },
    // ----- Geçici / log -----
    Rule {
        id: "ext-log",
        pattern: Pattern::Extension(".log"),
        score: 88,
        explanation: "Log dosyası — uygulama yeniden yazar.",
    },
    Rule {
        id: "ext-tmp",
        pattern: Pattern::Extension(".tmp"),
        score: 95,
        explanation: "Geçici dosya — silinmesi güvenli.",
    },
    Rule {
        id: "ext-temp",
        pattern: Pattern::Extension(".temp"),
        score: 95,
        explanation: "Geçici dosya — silinmesi güvenli.",
    },
    Rule {
        id: "ext-bak",
        pattern: Pattern::Extension(".bak"),
        score: 70,
        explanation: "Yedek dosya — orijinal hâlâ varsa silinebilir.",
    },
    Rule {
        id: "ext-cache",
        pattern: Pattern::Extension(".cache"),
        score: 92,
        explanation: "Cache dosyası — yeniden üretilir.",
    },
    Rule {
        id: "ext-old",
        pattern: Pattern::Extension(".old"),
        score: 70,
        explanation: "Eski sürüm yedeği — değerlendirin.",
    },
    // ----- Windows özel -----
    Rule {
        id: "hiberfil",
        pattern: Pattern::Name("hiberfil.sys"),
        score: 50,
        explanation:
            "Hibernate dosyası — hibernate'i kapatırsanız silebilirsiniz (`powercfg /h off`).",
    },
    Rule {
        id: "windows-old",
        pattern: Pattern::Name("Windows.old"),
        score: 85,
        explanation: "Eski Windows kurulumu yedeği — geri dönüş gerekmiyorsa silinebilir.",
    },
    Rule {
        id: "softwaredist",
        pattern: Pattern::Name("SoftwareDistribution"),
        score: 70,
        explanation: "Windows Update indirme cache — yeniden indirilir.",
    },
    Rule {
        id: "winsxs-backup",
        pattern: Pattern::Name("Temp"),
        score: 85,
        explanation: "Genel Temp klasörü — büyük çoğunluğu silinebilir.",
    },
    Rule {
        id: "recyclebin",
        pattern: Pattern::Name("$RECYCLE.BIN"),
        score: 88,
        explanation: "Geri Dönüşüm Kutusu — kullanıcı zaten sildiği şeyleri tutar.",
    },
    // ----- Kullanıcı verisi koruma (LOW score = DOKUNMA) -----
    Rule {
        id: "documents",
        pattern: Pattern::Name("Documents"),
        score: 5,
        explanation: "Kullanıcı dokümanları — DOKUNMA.",
    },
    Rule {
        id: "desktop",
        pattern: Pattern::Name("Desktop"),
        score: 8,
        explanation: "Masaüstü — kullanıcı verisi.",
    },
    Rule {
        id: "pictures",
        pattern: Pattern::Name("Pictures"),
        score: 5,
        explanation: "Kullanıcı görselleri — DOKUNMA.",
    },
    Rule {
        id: "videos",
        pattern: Pattern::Name("Videos"),
        score: 5,
        explanation: "Kullanıcı videoları — DOKUNMA.",
    },
    Rule {
        id: "music",
        pattern: Pattern::Name("Music"),
        score: 8,
        explanation: "Kullanıcı müzikleri.",
    },
    Rule {
        id: "downloads",
        pattern: Pattern::Name("Downloads"),
        score: 35,
        explanation: "İndirilenler — gözden geçirilmesi gerekir, otomatik silmeyin.",
    },
];

/// Tek bir kuralı isim+is_dir üzerinde uygular. Match → Some, yok → None.
pub fn match_rule(name: &str, is_dir: bool) -> Option<Rule> {
    let name_lower = name.to_ascii_lowercase();
    for rule in RULES {
        match rule.pattern {
            Pattern::Name(target) => {
                if name_lower == target.to_ascii_lowercase() {
                    return Some(*rule);
                }
            }
            Pattern::Extension(ext) => {
                if !is_dir && name_lower.ends_with(&ext.to_ascii_lowercase()) {
                    return Some(*rule);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_node_modules() {
        let r = match_rule("node_modules", true).expect("eşleşmeli");
        assert_eq!(r.id, "node_modules");
        assert_eq!(r.score, 95);
    }

    #[test]
    fn matches_extension_case_insensitive() {
        let r = match_rule("app.LOG", false).expect("eşleşmeli");
        assert_eq!(r.id, "ext-log");
    }

    #[test]
    fn extension_does_not_match_dir() {
        let r = match_rule("foo.log", true); // is_dir=true → uzantı atla
        assert!(r.is_none());
    }

    #[test]
    fn user_data_low_score() {
        let r = match_rule("Documents", true).expect("eşleşmeli");
        assert!(r.score < 30, "kullanıcı verisi düşük skorlu");
    }

    #[test]
    fn unknown_returns_none() {
        assert!(match_rule("randomname12345", false).is_none());
        assert!(match_rule("randomdir12345", true).is_none());
    }
}
