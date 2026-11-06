//! Zero-allocation localization and translation dictionary for Headroom.
//!
//! # Languages Supported
//! - English (en-US, Default)
//! - German (de-DE)
//! - Spanish (es-ES)
//! - French (fr-FR)
//! - Simplified Chinese (zh-CN)

use serde::{Deserialize, Serialize};

/// Supported interface languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    EnUs,
    DeDe,
    EsEs,
    FrFr,
    ZhCn,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::DeDe => "de-DE",
            Self::EsEs => "es-ES",
            Self::FrFr => "fr-FR",
            Self::ZhCn => "zh-CN",
        }
    }
}

/// Identifiers for localized user interface strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextId {
    StateHealthy,
    StateCacheFull,
    StateUnderPressure,
    StateCommitRisk,
    ModeMonitor,
    ModeAskFirst,
    ModeAutomatic,
    OpenDashboard,
    ExitHeadroom,
    StutterProfile,
    TrimBackgroundNow,
    PurgeStandbyNow,
}

/// Returns localized string slice for the given text identifier and language.
pub fn tr(id: TextId, lang: Language) -> &'static str {
    match lang {
        Language::EnUs => match id {
            TextId::StateHealthy => "Healthy",
            TextId::StateCacheFull => "Cache Full (Normal)",
            TextId::StateUnderPressure => "Under Pressure",
            TextId::StateCommitRisk => "Commit Risk",
            TextId::ModeMonitor => "Mode: Monitor Only",
            TextId::ModeAskFirst => "Mode: Ask First",
            TextId::ModeAutomatic => "Mode: Automatic Clean",
            TextId::OpenDashboard => "Open Dashboard",
            TextId::ExitHeadroom => "Exit Headroom",
            TextId::StutterProfile => "Stutter Profile (Gaming)",
            TextId::TrimBackgroundNow => "Trim Background Apps Now",
            TextId::PurgeStandbyNow => "Purge Low Standby Now",
        },
        Language::DeDe => match id {
            TextId::StateHealthy => "Optimal",
            TextId::StateCacheFull => "Cache voll (Normal)",
            TextId::StateUnderPressure => "Speicherdruck",
            TextId::StateCommitRisk => "Commit-Risiko",
            TextId::ModeMonitor => "Modus: Nur Überwachung",
            TextId::ModeAskFirst => "Modus: Nachfragen",
            TextId::ModeAutomatic => "Modus: Automatisch",
            TextId::OpenDashboard => "Dashboard öffnen",
            TextId::ExitHeadroom => "Headroom beenden",
            TextId::StutterProfile => "Ruckel-Profil (Gaming)",
            TextId::TrimBackgroundNow => "Hintergrund-Apps leeren",
            TextId::PurgeStandbyNow => "Standby-Cache bereinigen",
        },
        Language::EsEs => match id {
            TextId::StateHealthy => "Saludable",
            TextId::StateCacheFull => "Caché llena (Normal)",
            TextId::StateUnderPressure => "Bajo presión",
            TextId::StateCommitRisk => "Riesgo de Commit",
            TextId::ModeMonitor => "Modo: Solo monitor",
            TextId::ModeAskFirst => "Modo: Preguntar primero",
            TextId::ModeAutomatic => "Modo: Limpieza automática",
            TextId::OpenDashboard => "Abrir panel de control",
            TextId::ExitHeadroom => "Salir de Headroom",
            TextId::StutterProfile => "Perfil de tirones (Juegos)",
            TextId::TrimBackgroundNow => "Ajustar apps en segundo plano",
            TextId::PurgeStandbyNow => "Purgar caché en espera",
        },
        Language::FrFr => match id {
            TextId::StateHealthy => "Sain",
            TextId::StateCacheFull => "Cache plein (Normal)",
            TextId::StateUnderPressure => "Sous pression",
            TextId::StateCommitRisk => "Risque de dépassement",
            TextId::ModeMonitor => "Mode : Surveillance seule",
            TextId::ModeAskFirst => "Mode : Demander d'abord",
            TextId::ModeAutomatic => "Mode : Nettoyage automatique",
            TextId::OpenDashboard => "Ouvrir le tableau de bord",
            TextId::ExitHeadroom => "Quitter Headroom",
            TextId::StutterProfile => "Profil anti-saccades (Jeux)",
            TextId::TrimBackgroundNow => "Optimiser l'arrière-plan",
            TextId::PurgeStandbyNow => "Purger la mémoire en veille",
        },
        Language::ZhCn => match id {
            TextId::StateHealthy => "运行良好",
            TextId::StateCacheFull => "缓存已满 (正常)",
            TextId::StateUnderPressure => "内存吃紧",
            TextId::StateCommitRisk => "提交量过载风险",
            TextId::ModeMonitor => "模式：仅监控",
            TextId::ModeAskFirst => "模式：清理前询问",
            TextId::ModeAutomatic => "模式：自动清理",
            TextId::OpenDashboard => "打开诊断面板",
            TextId::ExitHeadroom => "退出 Headroom",
            TextId::StutterProfile => "防卡顿模式 (游戏优化)",
            TextId::TrimBackgroundNow => "立即精简后台应用",
            TextId::PurgeStandbyNow => "清理低优先级待机缓存",
        },
    }
}
