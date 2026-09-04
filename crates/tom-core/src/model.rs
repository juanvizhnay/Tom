use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum AiProvider {
    #[default]
    Disabled,
    OpenAi,
    Anthropic,
    Gemini,
    CustomCloud,
    Ollama,
    LmStudio,
    CustomLocal,
}

impl AiProvider {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::CustomCloud => "custom-cloud",
            Self::Ollama => "ollama",
            Self::LmStudio => "lm-studio",
            Self::CustomLocal => "custom-local",
        }
    }

    #[must_use]
    pub fn from_stable_id(stable_id: &str) -> Option<Self> {
        match stable_id {
            "disabled" => Some(Self::Disabled),
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            "gemini" => Some(Self::Gemini),
            "custom-cloud" => Some(Self::CustomCloud),
            "ollama" => Some(Self::Ollama),
            "lm-studio" => Some(Self::LmStudio),
            "custom-local" => Some(Self::CustomLocal),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AiSettings {
    pub provider: AiProvider,
    pub endpoint: String,
    pub model_path: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Persona {
    #[default]
    Tom,
    Tomy,
}

impl Persona {
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Tom => "Tom",
            Self::Tomy => "Tomy",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProfessionalProfile {
    #[default]
    Developer,
    Office,
    Student,
    Creator,
    General,
}

impl ProfessionalProfile {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Developer => "developer",
            Self::Office => "office",
            Self::Student => "student",
            Self::Creator => "creator",
            Self::General => "general",
        }
    }

    #[must_use]
    pub fn from_stable_id(value: &str) -> Option<Self> {
        match value {
            "developer" => Some(Self::Developer),
            "office" => Some(Self::Office),
            "student" => Some(Self::Student),
            "creator" => Some(Self::Creator),
            "general" => Some(Self::General),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum AssistanceStyle {
    Focused,
    #[default]
    Balanced,
    Proactive,
}

impl AssistanceStyle {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Focused => "focused",
            Self::Balanced => "balanced",
            Self::Proactive => "proactive",
        }
    }

    #[must_use]
    pub fn from_stable_id(value: &str) -> Option<Self> {
        match value {
            "focused" => Some(Self::Focused),
            "balanced" => Some(Self::Balanced),
            "proactive" => Some(Self::Proactive),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct UserProfile {
    pub persona: Persona,
    pub professional_profile: ProfessionalProfile,
    pub assistance_style: AssistanceStyle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Note {
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            title: title.into(),
            body: body.into(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoutineSuggestion {
    pub id: Uuid,
    pub title: String,
    pub rationale: String,
    pub proposed_action: String,
    pub created_at: DateTime<Utc>,
}

impl RoutineSuggestion {
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        rationale: impl Into<String>,
        proposed_action: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            title: title.into(),
            rationale: rationale.into(),
            proposed_action: proposed_action.into(),
            created_at: Utc::now(),
        }
    }
}
