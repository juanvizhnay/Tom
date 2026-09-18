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
    /// Every provider the assistant knows about, in the order the interface offers them.
    pub const ALL: [Self; 8] = [
        Self::Disabled,
        Self::OpenAi,
        Self::Anthropic,
        Self::Gemini,
        Self::CustomCloud,
        Self::Ollama,
        Self::LmStudio,
        Self::CustomLocal,
    ];

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
        Self::ALL
            .into_iter()
            .find(|provider| provider.stable_id() == stable_id)
    }

    /// Whether the provider runs outside this device and therefore needs a credential.
    #[must_use]
    pub const fn is_cloud(self) -> bool {
        matches!(
            self,
            Self::OpenAi | Self::Anthropic | Self::Gemini | Self::CustomCloud
        )
    }

    /// Whether the provider is expected to answer on this machine.
    #[must_use]
    pub const fn is_local(self) -> bool {
        matches!(self, Self::Ollama | Self::LmStudio | Self::CustomLocal)
    }

    /// Whether a credential can be stored for the provider.
    ///
    /// No single key is ever required: a cloud provider is reached with one, a local server
    /// may want one if it sits behind a proxy, and the vault holds whichever the user
    /// chooses to add.
    #[must_use]
    pub const fn accepts_api_key(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Whether the user must supply an endpoint before the provider can be reached.
    #[must_use]
    pub const fn requires_endpoint(self) -> bool {
        matches!(self, Self::CustomCloud)
    }

    /// Whether the provider needs either an endpoint or a model path to be usable.
    #[must_use]
    pub const fn requires_model_location(self) -> bool {
        matches!(self, Self::CustomLocal)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AiSettings {
    pub provider: AiProvider,
    pub endpoint: String,
    pub model_path: String,
}

impl AiSettings {
    /// Whether a local model is described well enough for Tom to reach it.
    #[must_use]
    pub fn has_reachable_local_model(&self) -> bool {
        self.provider.is_local()
            && (!self.endpoint.trim().is_empty() || !self.model_path.trim().is_empty())
    }

    /// Whether Tom has any way at all to think.
    ///
    /// Nothing in particular is required: one stored key, from any provider, or one local
    /// model it can reach, is enough. Having none of either is the single configuration
    /// Tom cannot work with, and the interface says so rather than failing quietly.
    #[must_use]
    pub fn intelligence_is_available(&self, stored_key_count: usize) -> bool {
        stored_key_count > 0 || self.has_reachable_local_model()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Persona {
    #[default]
    Tom,
    Tomy,
}

impl Persona {
    pub const ALL: [Self; 2] = [Self::Tom, Self::Tomy];

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Tom => "Tom",
            Self::Tomy => "Tomy",
        }
    }

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Tom => "tom",
            Self::Tomy => "tomy",
        }
    }

    #[must_use]
    pub fn from_stable_id(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|persona| persona.stable_id() == value)
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
    pub const ALL: [Self; 5] = [
        Self::Developer,
        Self::Office,
        Self::Student,
        Self::Creator,
        Self::General,
    ];

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
        Self::ALL
            .into_iter()
            .find(|profile| profile.stable_id() == value)
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
    pub const ALL: [Self; 3] = [Self::Focused, Self::Balanced, Self::Proactive];

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
        Self::ALL
            .into_iter()
            .find(|style| style.stable_id() == value)
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
