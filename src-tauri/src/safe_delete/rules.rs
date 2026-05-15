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

/// Bölüm 6.2 — built-in rule seti. Faz 2 v0.3: 63 kural.
/// Eşit isimli kural eşleşirse ilk eşleşen kullanılır (sıra önemli).
/// **System safety**: pagefile/swapfile/boot dosyaları LOW score (DOKUNMA);
/// `match_rule` orijinal isim eşleşmesinden döner — Bölüm 34.5.1 sistem
/// dosyalarına temas etmemenin tamamlayıcı katmanı.
/// **Cloud + VM safety**: OneDrive/Dropbox/iCloud + WSL/Docker/Hyper-V
/// klasörleri kullanıcı verisi olarak işaretlenir (Bölüm 10 + 11 v0.1).
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
    Rule {
        id: "pytest-cache",
        pattern: Pattern::Name(".pytest_cache"),
        score: 95,
        explanation: "Pytest cache — sonraki çalıştırmada yeniden üretilir.",
    },
    Rule {
        id: "mypy-cache",
        pattern: Pattern::Name(".mypy_cache"),
        score: 95,
        explanation: "Mypy type cache — yeniden üretilir.",
    },
    Rule {
        id: "ruff-cache",
        pattern: Pattern::Name(".ruff_cache"),
        score: 95,
        explanation: "Ruff linter cache — yeniden üretilir.",
    },
    Rule {
        id: "parcel-cache",
        pattern: Pattern::Name(".parcel-cache"),
        score: 92,
        explanation: "Parcel bundler cache — yeniden üretilir.",
    },
    Rule {
        id: "turbo-cache",
        pattern: Pattern::Name(".turbo"),
        score: 90,
        explanation: "Turborepo cache — yeniden üretilir.",
    },
    Rule {
        id: "terraform-cache",
        pattern: Pattern::Name(".terraform"),
        score: 75,
        explanation: "Terraform plugin cache — `terraform init` yeniden indirir.",
    },
    Rule {
        id: "coverage",
        pattern: Pattern::Name("coverage"),
        score: 80,
        explanation: "Test coverage raporu — testler yeniden çalıştırılırsa gelir.",
    },
    Rule {
        id: "obj-dotnet",
        pattern: Pattern::Name("obj"),
        score: 75,
        explanation: ".NET intermediate build artifact — `dotnet build` ile gelir.",
    },
    Rule {
        id: "idea-config",
        pattern: Pattern::Name(".idea"),
        score: 45,
        explanation: "JetBrains IDE projesi ayarları — değerli kişisel yapılandırma içerebilir.",
    },
    Rule {
        id: "vs-config",
        pattern: Pattern::Name(".vs"),
        score: 50,
        explanation: "Visual Studio çalışma alanı durumu — yeniden açıldığında kurulur.",
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
    Rule {
        id: "cache-data",
        pattern: Pattern::Name("Cache_Data"),
        score: 90,
        explanation: "Discord/Electron uygulama cache — yeniden üretilir.",
    },
    Rule {
        id: "service-worker",
        pattern: Pattern::Name("Service Worker"),
        score: 85,
        explanation: "PWA service worker cache — yeniden indirilir.",
    },
    Rule {
        id: "msocache",
        pattern: Pattern::Name("MSOCache"),
        score: 75,
        explanation: "Office installer cache — yeniden indirilebilir.",
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
    Rule {
        id: "ext-dmp",
        pattern: Pattern::Extension(".dmp"),
        score: 85,
        explanation: "Crash dump dosyası — kritik veri değil.",
    },
    Rule {
        id: "ext-etl",
        pattern: Pattern::Extension(".etl"),
        score: 88,
        explanation: "Windows event trace log — eski performans izleri.",
    },
    Rule {
        id: "ext-swp",
        pattern: Pattern::Extension(".swp"),
        score: 90,
        explanation: "Vim swap dosyası — vim açık değilse silinebilir.",
    },
    Rule {
        id: "ext-swo",
        pattern: Pattern::Extension(".swo"),
        score: 90,
        explanation: "Vim swap dosyası (ek) — silinebilir.",
    },
    Rule {
        id: "ext-pdb",
        pattern: Pattern::Extension(".pdb"),
        score: 55,
        explanation: ".NET/C++ debug sembol dosyası — yeniden derleme ile gelir, ama mevcut binary debug'unu kaybedersin.",
    },
    Rule {
        id: "ext-ilk",
        pattern: Pattern::Extension(".ilk"),
        score: 88,
        explanation: "MSVC incremental link cache — yeniden derleme ile üretilir.",
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
    // ----- System dosyaları — DOKUNMA (kullanıcıyı koru) -----
    Rule {
        id: "pagefile",
        pattern: Pattern::Name("pagefile.sys"),
        score: 3,
        explanation: "Windows sayfa dosyası — sistem yönetir, asla manuel silme.",
    },
    Rule {
        id: "swapfile",
        pattern: Pattern::Name("swapfile.sys"),
        score: 3,
        explanation: "Windows swap dosyası — sistem yönetir.",
    },
    Rule {
        id: "bootmgr",
        pattern: Pattern::Name("bootmgr"),
        score: 1,
        explanation: "Windows boot manager — silersen sistem açılmaz.",
    },
    Rule {
        id: "bootnxt",
        pattern: Pattern::Name("BOOTNXT"),
        score: 1,
        explanation: "Windows boot config — silersen önyükleme bozulur.",
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
    Rule {
        id: "appdata-roaming",
        pattern: Pattern::Name("AppData"),
        score: 25,
        explanation: "Uygulama verisi — ayar/profil içerir, dikkat.",
    },
    // ----- Cloud sync klasörleri (Bölüm 11) — LOW score, kullanıcı verisi -----
    Rule {
        id: "onedrive",
        pattern: Pattern::Name("OneDrive"),
        score: 10,
        explanation: "OneDrive senkron klasörü — silmek bulut sürümünü de etkileyebilir.",
    },
    Rule {
        id: "onedrive-business",
        pattern: Pattern::Name("OneDrive - Personal"),
        score: 10,
        explanation: "OneDrive (kişisel) — bulut sürümünü etkiler.",
    },
    Rule {
        id: "dropbox",
        pattern: Pattern::Name("Dropbox"),
        score: 10,
        explanation: "Dropbox senkron klasörü — silmek buluttan da kaldırabilir.",
    },
    Rule {
        id: "google-drive",
        pattern: Pattern::Name("Google Drive"),
        score: 10,
        explanation: "Google Drive senkron klasörü — bulut sürümünü etkiler.",
    },
    Rule {
        id: "icloud-drive",
        pattern: Pattern::Name("iCloudDrive"),
        score: 10,
        explanation: "iCloud Drive senkron klasörü — bulut sürümünü etkiler.",
    },
    // ----- VM / WSL / container imaj dosyaları (Bölüm 10) — LOW score -----
    Rule {
        id: "wsl-root",
        pattern: Pattern::Name(".wslconfig"),
        score: 8,
        explanation: "WSL yapılandırma dosyası — silmek tüm WSL dağıtımlarını etkiler.",
    },
    Rule {
        id: "ext-vhdx",
        pattern: Pattern::Extension(".vhdx"),
        score: 5,
        explanation: "Sanal disk (Hyper-V/WSL/Docker) — silmek dağıtımı yok eder.",
    },
    Rule {
        id: "ext-vhd",
        pattern: Pattern::Extension(".vhd"),
        score: 5,
        explanation: "Sanal disk (eski VHD format) — silmek dağıtımı yok eder.",
    },
    Rule {
        id: "ext-vdi",
        pattern: Pattern::Extension(".vdi"),
        score: 5,
        explanation: "VirtualBox sanal disk — silmek VM'i yok eder.",
    },
    Rule {
        id: "ext-vmdk",
        pattern: Pattern::Extension(".vmdk"),
        score: 5,
        explanation: "VMware sanal disk — silmek VM'i yok eder.",
    },
    Rule {
        id: "ext-qcow2",
        pattern: Pattern::Extension(".qcow2"),
        score: 5,
        explanation: "QEMU/KVM sanal disk — silmek VM'i yok eder.",
    },
    Rule {
        id: "ext-ova",
        pattern: Pattern::Extension(".ova"),
        score: 40,
        explanation: "OVA bundle — VM şablonu, çoğunlukla import sonrası silinebilir.",
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

    #[test]
    fn rule_count_meets_spec_target() {
        // Bölüm 6.2 — "50+ kural" hedefi.
        assert!(
            RULES.len() >= 50,
            "Spec 50+ kural istiyor, mevcut {}",
            RULES.len()
        );
    }

    #[test]
    fn cloud_sync_folders_protected() {
        // Bölüm 11 — cloud sync klasörleri kullanıcı verisi, LOW score.
        for name in ["OneDrive", "Dropbox", "Google Drive", "iCloudDrive"] {
            let r = match_rule(name, true).unwrap_or_else(|| panic!("{} eşleşmeli", name));
            assert!(
                r.score <= 30,
                "{} cloud klasörü düşük skor olmalı (got {})",
                name,
                r.score
            );
        }
    }

    #[test]
    fn vm_disk_extensions_protected() {
        // Bölüm 10 — sanal disk uzantıları LOW score (WSL/Docker/Hyper-V/VirtualBox/VMware).
        for ext in ["disk.vhdx", "vm.vdi", "container.vmdk", "image.qcow2"] {
            let r = match_rule(ext, false).unwrap_or_else(|| panic!("{} eşleşmeli", ext));
            assert!(
                r.score <= 30,
                "{} sanal disk düşük skor olmalı (got {})",
                ext,
                r.score
            );
        }
    }

    #[test]
    fn system_files_protected_with_low_score() {
        // pagefile/swapfile/bootmgr → DOKUNMA tier (skor ≤ 30)
        for name in ["pagefile.sys", "swapfile.sys", "bootmgr", "BOOTNXT"] {
            let r = match_rule(name, false).unwrap_or_else(|| panic!("{} eşleşmeli", name));
            assert!(
                r.score <= 10,
                "{} sistem dosyası çok düşük skor olmalı (got {})",
                name,
                r.score
            );
        }
    }

    #[test]
    fn crash_and_temp_extensions_high_score() {
        for (name, expected_min) in [
            ("error.dmp", 80),
            ("trace.etl", 80),
            (".session.swp", 80),
            ("file.tmp", 90),
        ] {
            let r = match_rule(name, false).unwrap_or_else(|| panic!("{} eşleşmeli", name));
            assert!(
                r.score >= expected_min,
                "{} → skor {} (>= {} bekleniyordu)",
                name,
                r.score,
                expected_min
            );
        }
    }

    #[test]
    fn dev_caches_recognized() {
        for name in [
            ".pytest_cache",
            ".mypy_cache",
            ".ruff_cache",
            ".parcel-cache",
            ".turbo",
            ".terraform",
            "coverage",
            "obj",
        ] {
            assert!(
                match_rule(name, true).is_some(),
                "{} dev cache kuralı bekleniyor",
                name
            );
        }
    }
}
