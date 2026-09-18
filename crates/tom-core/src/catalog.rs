//! The copy and option lists the desktop shell renders.
//!
//! Every option the interface can show is derived here from the domain enums, so an id
//! that reaches a callback is always an id that parses back into a domain value. Keeping
//! the catalog next to the model is what stops the two from drifting apart: adding a
//! provider or a professional profile is a single edit, and the tests below fail if a
//! label, a detail, or a stable id is left behind.

use crate::{AiProvider, AssistanceStyle, Persona, ProfessionalProfile};

/// One selectable option: a stable id plus the copy that explains it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Choice {
    pub id: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
}

impl Persona {
    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            Self::Tom => {
                "Sereno, compacto y directo. Ideal cuando quieres foco y lenguaje profesional."
            }
            Self::Tomy => {
                "Cercano, cálido y alentador. El mismo criterio con una energía más relajada."
            }
        }
    }

    #[must_use]
    pub const fn as_choice(self) -> Choice {
        Choice {
            id: self.stable_id(),
            label: self.display_name(),
            detail: self.detail(),
        }
    }
}

impl ProfessionalProfile {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Developer => "Desarrollo",
            Self::Office => "Oficina",
            Self::Student => "Estudio",
            Self::Creator => "Creación",
            Self::General => "Uso general",
        }
    }

    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            Self::Developer => "Código, Git, terminal y proyectos",
            Self::Office => "Documentos, reuniones y seguimiento",
            Self::Student => "Lecturas, entregas y aprendizaje",
            Self::Creator => "Ideas, contenido y producción",
            Self::General => "Organización diaria sin una profesión dominante",
        }
    }

    #[must_use]
    pub const fn as_choice(self) -> Choice {
        Choice {
            id: self.stable_id(),
            label: self.label(),
            detail: self.detail(),
        }
    }
}

impl AssistanceStyle {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Focused => "Enfocado",
            Self::Balanced => "Equilibrado",
            Self::Proactive => "Proactivo",
        }
    }

    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            Self::Focused => "Sólo responde y recuerda cuando se lo pidas.",
            Self::Balanced => "Sugiere ayudas cuando el patrón es claro, sin interrumpir.",
            Self::Proactive => "Busca oportunidades y prepara acciones para que tú las apruebes.",
        }
    }

    #[must_use]
    pub const fn as_choice(self) -> Choice {
        Choice {
            id: self.stable_id(),
            label: self.label(),
            detail: self.detail(),
        }
    }
}

impl AiProvider {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "Sin proveedor",
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::Gemini => "Gemini",
            Self::CustomCloud => "Cloud compatible",
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::CustomLocal => "Local personalizado",
        }
    }

    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            Self::Disabled => "Tom funciona sólo con tu memoria local",
            Self::OpenAi => "API key propia",
            Self::Anthropic => "Claude con tu clave",
            Self::Gemini => "Google AI",
            Self::CustomCloud => "Endpoint personalizado",
            Self::Ollama => "Local · endpoint automático",
            Self::LmStudio => "Servidor local compatible",
            Self::CustomLocal => "Endpoint o ruta de modelo",
        }
    }

    /// Label for the endpoint field, which means different things on-device and in the cloud.
    #[must_use]
    pub const fn endpoint_label(self) -> &'static str {
        if self.is_local() {
            "Endpoint local"
        } else {
            "Endpoint compatible (opcional)"
        }
    }

    #[must_use]
    pub const fn endpoint_placeholder(self) -> &'static str {
        match self {
            Self::Ollama => "http://127.0.0.1:11434",
            Self::LmStudio => "http://127.0.0.1:1234/v1",
            _ => "https://… o http://127.0.0.1:…",
        }
    }

    #[must_use]
    pub const fn as_choice(self) -> Choice {
        Choice {
            id: self.stable_id(),
            label: self.label(),
            detail: self.detail(),
        }
    }
}

/// Providers the key manager can hold a credential for, in the order it lists them.
#[must_use]
pub fn api_key_providers() -> Vec<AiProvider> {
    AiProvider::ALL
        .into_iter()
        .filter(|provider| provider.accepts_api_key())
        .collect()
}

/// Providers offered under the "cloud" heading of the settings panel.
#[must_use]
pub fn cloud_provider_choices() -> Vec<Choice> {
    choices_where(AiProvider::is_cloud)
}

/// Providers offered under the "local" heading of the settings panel.
#[must_use]
pub fn local_provider_choices() -> Vec<Choice> {
    choices_where(AiProvider::is_local)
}

fn choices_where(predicate: fn(AiProvider) -> bool) -> Vec<Choice> {
    AiProvider::ALL
        .into_iter()
        .filter(|provider| predicate(*provider))
        .map(AiProvider::as_choice)
        .collect()
}

/// How the user wants to connect intelligence when leaving onboarding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AiSetup {
    #[default]
    Later,
    Cloud,
    Local,
}

impl AiSetup {
    pub const ALL: [Self; 3] = [Self::Later, Self::Cloud, Self::Local];

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Later => "later",
            Self::Cloud => "cloud",
            Self::Local => "local",
        }
    }

    #[must_use]
    pub fn from_stable_id(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|setup| setup.stable_id() == value)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Later => "Configurar después",
            Self::Cloud => "Proveedor cloud",
            Self::Local => "Modelo local",
        }
    }

    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            Self::Later => "Entra a Tom sin proveedor; memoria y notas siguen disponibles.",
            Self::Cloud => "OpenAI, Anthropic, Gemini o endpoint compatible con tu API key.",
            Self::Local => "Ollama, LM Studio o un endpoint/modelo propio en este PC.",
        }
    }

    /// The provider the settings panel should pre-select once onboarding ends.
    ///
    /// [`AiSetup::Later`] resolves to [`AiProvider::Disabled`], which is also the signal the
    /// desktop shell uses to decide whether the panel opens at all.
    #[must_use]
    pub const fn preselected_provider(self) -> AiProvider {
        match self {
            Self::Later => AiProvider::Disabled,
            Self::Cloud => AiProvider::OpenAi,
            Self::Local => AiProvider::Ollama,
        }
    }

    #[must_use]
    pub const fn as_choice(self) -> Choice {
        Choice {
            id: self.stable_id(),
            label: self.label(),
            detail: self.detail(),
        }
    }
}

/// The five questions Tom asks before opening the workspace.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OnboardingStep {
    #[default]
    Welcome,
    PersonaChoice,
    ProfessionChoice,
    AssistanceChoice,
    IntelligenceChoice,
}

impl OnboardingStep {
    pub const ALL: [Self; 5] = [
        Self::Welcome,
        Self::PersonaChoice,
        Self::ProfessionChoice,
        Self::AssistanceChoice,
        Self::IntelligenceChoice,
    ];

    /// The last step, whose primary button finishes onboarding instead of advancing.
    pub const LAST: Self = Self::IntelligenceChoice;

    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    #[must_use]
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|step| *step == self)
            .unwrap_or_default()
    }

    /// The step after this one, or `None` when there is nothing left to ask.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        Self::from_index(self.index() + 1)
    }

    /// The step before this one, or `None` on the first question.
    #[must_use]
    pub fn previous(self) -> Option<Self> {
        self.index().checked_sub(1).and_then(Self::from_index)
    }

    /// Short name shown in the onboarding progress rail.
    #[must_use]
    pub const fn rail_label(self) -> &'static str {
        match self {
            Self::Welcome => "Hola",
            Self::PersonaChoice => "Personalidad",
            Self::ProfessionChoice => "Tu mundo",
            Self::AssistanceChoice => "Iniciativa",
            Self::IntelligenceChoice => "Inteligencia",
        }
    }

    #[must_use]
    pub const fn headline(self) -> &'static str {
        match self {
            Self::Welcome => "Hola. Soy Tom.",
            Self::PersonaChoice => "¿Cómo quieres que hable contigo?",
            Self::ProfessionChoice => "¿En qué mundo trabajaremos juntos?",
            Self::AssistanceChoice => "¿Cuánta iniciativa quieres de mí?",
            Self::IntelligenceChoice => "¿Qué inteligencia quieres conectar?",
        }
    }

    #[must_use]
    pub const fn body(self) -> &'static str {
        match self {
            Self::Welcome => {
                "Antes de mostrarte paneles, quiero entender cómo acompañarte. Son cuatro decisiones cortas y podrás cambiarlas después."
            }
            Self::PersonaChoice => {
                "Tom es más preciso y profesional. Tomy conserva el mismo criterio, con una voz más cercana y chill."
            }
            Self::ProfessionChoice => {
                "Esto activa vocabulario, rutinas y sugerencias específicas para tu actividad principal."
            }
            Self::AssistanceChoice => {
                "Nunca ejecutaré acciones sensibles sin aprobación, sin importar el nivel elegido."
            }
            Self::IntelligenceChoice => {
                "La clave de nube vivirá en Windows Credential Manager. Los modelos locales pueden funcionar sin enviar contenido fuera del PC."
            }
        }
    }

    /// One line under the orb, spoken in the assistant's own voice.
    #[must_use]
    pub const fn orb_caption(self) -> &'static str {
        match self {
            Self::Welcome => "Una presencia tranquila para tu escritorio.",
            Self::PersonaChoice => "Mi voz se adapta; tus controles no cambian.",
            Self::ProfessionChoice => "Aprenderé el contexto de tu trabajo.",
            Self::AssistanceChoice => "Tú decides cuánto puedo anticiparme.",
            Self::IntelligenceChoice => "Puedes conectar una IA ahora o después.",
        }
    }

    /// Orb state this step should force, or `None` to keep whatever the assistant is showing.
    ///
    /// The returned id is always one of the orb's stable state ids.
    #[must_use]
    pub const fn orb_state_id(self) -> Option<&'static str> {
        match self {
            Self::Welcome => Some("curious"),
            Self::IntelligenceChoice => Some("thinking"),
            _ => None,
        }
    }

    #[must_use]
    pub const fn primary_label(self) -> &'static str {
        match self {
            Self::Welcome => "Conocernos",
            Self::IntelligenceChoice => "Terminar",
            _ => "Continuar",
        }
    }

    /// The options this step asks the user to pick from; empty on the welcome step.
    #[must_use]
    pub fn choices(self) -> Vec<Choice> {
        match self {
            Self::Welcome => Vec::new(),
            Self::PersonaChoice => Persona::ALL.into_iter().map(Persona::as_choice).collect(),
            Self::ProfessionChoice => ProfessionalProfile::ALL
                .into_iter()
                .map(ProfessionalProfile::as_choice)
                .collect(),
            Self::AssistanceChoice => AssistanceStyle::ALL
                .into_iter()
                .map(AssistanceStyle::as_choice)
                .collect(),
            Self::IntelligenceChoice => AiSetup::ALL.into_iter().map(AiSetup::as_choice).collect(),
        }
    }
}

/// The promises shown on the welcome step, numbered in the interface.
#[must_use]
pub const fn welcome_highlights() -> [&'static str; 4] {
    [
        "Memoria visible y editable",
        "Rutinas explicadas antes de actuar",
        "IA local o cloud elegida por ti",
        "Un orbe discreto cuando no necesites el panel",
    ]
}

/// How a pulse entry is colored: what was seen, what was inferred, what is proposed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PulseTone {
    Observed,
    Inferred,
    Proposed,
}

impl PulseTone {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Inferred => "inferred",
            Self::Proposed => "proposed",
        }
    }

    #[must_use]
    pub const fn stage_label(self) -> &'static str {
        match self {
            Self::Observed => "Observado",
            Self::Inferred => "Inferido",
            Self::Proposed => "Propuesto",
        }
    }
}

/// One rung of the explainability ladder shown in Today and Routines.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PulseEntry {
    pub tone: PulseTone,
    pub title: &'static str,
    pub detail: &'static str,
    pub time: &'static str,
}

/// The timeline that separates observation from inference from proposal.
///
/// Both surfaces that show a pulse render this same list, so the explanation a user reads in
/// Today is word for word the one they read in Routines.
#[must_use]
pub const fn pulse_timeline() -> [PulseEntry; 3] {
    [
        PulseEntry {
            tone: PulseTone::Observed,
            title: "Volviste al workspace de Tom",
            detail: "Sesión local detectada; no se leyó contenido privado.",
            time: "ahora",
        },
        PulseEntry {
            tone: PulseTone::Inferred,
            title: "Estás retomando contexto",
            detail: "Este patrón suele aparecer al comenzar un bloque de desarrollo.",
            time: "+2 s",
        },
        PulseEntry {
            tone: PulseTone::Proposed,
            title: "Preparar el último hilo",
            detail: "La acción espera tu aprobación y puede descartarse.",
            time: "lista",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        AiSetup, OnboardingStep, PulseTone, api_key_providers, cloud_provider_choices,
        local_provider_choices, pulse_timeline, welcome_highlights,
    };
    use crate::{AiProvider, AssistanceStyle, Persona, ProfessionalProfile};

    #[test]
    fn every_onboarding_choice_id_parses_back_into_a_domain_value() {
        for choice in OnboardingStep::PersonaChoice.choices() {
            assert!(
                Persona::from_stable_id(choice.id).is_some(),
                "{}",
                choice.id
            );
        }
        for choice in OnboardingStep::ProfessionChoice.choices() {
            assert!(
                ProfessionalProfile::from_stable_id(choice.id).is_some(),
                "{}",
                choice.id
            );
        }
        for choice in OnboardingStep::AssistanceChoice.choices() {
            assert!(
                AssistanceStyle::from_stable_id(choice.id).is_some(),
                "{}",
                choice.id
            );
        }
        for choice in OnboardingStep::IntelligenceChoice.choices() {
            assert!(
                AiSetup::from_stable_id(choice.id).is_some(),
                "{}",
                choice.id
            );
        }
    }

    #[test]
    fn every_step_offers_copy_and_the_welcome_step_asks_nothing() {
        for (index, step) in OnboardingStep::ALL.into_iter().enumerate() {
            assert_eq!(OnboardingStep::from_index(index), Some(step));
            assert!(!step.rail_label().is_empty());
            assert!(!step.headline().is_empty());
            assert!(!step.body().is_empty());
            assert!(!step.orb_caption().is_empty());
            assert!(!step.primary_label().is_empty());
        }
        assert!(OnboardingStep::Welcome.choices().is_empty());
        assert_eq!(OnboardingStep::from_index(OnboardingStep::ALL.len()), None);
    }

    #[test]
    fn only_the_last_step_finishes_onboarding() {
        assert_eq!(OnboardingStep::LAST, OnboardingStep::IntelligenceChoice);
        assert_eq!(OnboardingStep::LAST.primary_label(), "Terminar");
        assert_eq!(
            OnboardingStep::from_index(OnboardingStep::ALL.len() - 1),
            Some(OnboardingStep::LAST)
        );
        assert_eq!(OnboardingStep::LAST.next(), None);
        assert_eq!(OnboardingStep::default().previous(), None);
    }

    #[test]
    fn stepping_forward_and_back_walks_the_same_ladder() {
        let mut step = OnboardingStep::default();

        for expected in OnboardingStep::ALL.into_iter().skip(1) {
            step = step.next().expect("a step remains before the last one");
            assert_eq!(step, expected);
        }
        for expected in OnboardingStep::ALL.into_iter().rev().skip(1) {
            step = step.previous().expect("a step remains after the first one");
            assert_eq!(step, expected);
        }
        assert_eq!(step, OnboardingStep::default());
        assert_eq!(step.index(), 0);
    }

    #[test]
    fn welcome_highlights_are_distinct_promises() {
        let highlights = welcome_highlights();
        for (index, highlight) in highlights.iter().enumerate() {
            assert!(!highlight.is_empty());
            assert!(!highlights[..index].contains(highlight));
        }
    }

    #[test]
    fn provider_groups_cover_every_provider_exactly_once() {
        let cloud = cloud_provider_choices();
        let local = local_provider_choices();

        assert_eq!(cloud.len() + local.len() + 1, AiProvider::ALL.len());
        for choice in cloud.iter().chain(&local) {
            let provider =
                AiProvider::from_stable_id(choice.id).expect("catalog ids are provider ids");
            assert_ne!(provider, AiProvider::Disabled);
            assert!(provider.is_cloud() ^ provider.is_local());
        }
    }

    #[test]
    fn only_cloud_providers_are_reached_through_a_credential() {
        for provider in AiProvider::ALL {
            assert!(!(provider.is_cloud() && provider.is_local()));
            assert_eq!(
                provider.endpoint_label() == "Endpoint local",
                provider.is_local()
            );
        }
        assert!(AiProvider::CustomCloud.requires_endpoint());
        assert!(AiProvider::CustomLocal.requires_model_location());
        assert!(!AiProvider::Ollama.requires_model_location());
    }

    #[test]
    fn the_key_manager_lists_every_provider_except_the_disabled_one() {
        let providers = api_key_providers();

        assert_eq!(providers.len(), AiProvider::ALL.len() - 1);
        assert!(!providers.contains(&AiProvider::Disabled));
        for provider in providers {
            assert!(provider.accepts_api_key());
        }
        assert!(!AiProvider::Disabled.accepts_api_key());
    }

    #[test]
    fn ai_setup_preselects_a_provider_of_the_matching_kind() {
        assert_eq!(AiSetup::Later.preselected_provider(), AiProvider::Disabled);
        assert!(AiSetup::Cloud.preselected_provider().is_cloud());
        assert!(AiSetup::Local.preselected_provider().is_local());
        assert_eq!(AiSetup::from_stable_id("unknown"), None);
    }

    #[test]
    fn the_pulse_walks_from_observation_to_proposal() {
        let tones = pulse_timeline().map(|entry| entry.tone);

        assert_eq!(
            tones,
            [
                PulseTone::Observed,
                PulseTone::Inferred,
                PulseTone::Proposed
            ]
        );
        for entry in pulse_timeline() {
            assert!(!entry.title.is_empty());
            assert!(!entry.detail.is_empty());
            assert!(!entry.time.is_empty());
            assert!(!entry.tone.stage_label().is_empty());
            assert!(!entry.tone.stable_id().is_empty());
        }
    }
}
