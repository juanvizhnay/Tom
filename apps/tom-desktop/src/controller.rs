#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PersonaChoice {
    #[default]
    Tom,
    Tomy,
}

impl PersonaChoice {
    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("tomy") || value.eq_ignore_ascii_case("friendly") {
            Self::Tomy
        } else {
            Self::Tom
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Tom => "Tom",
            Self::Tomy => "Tomy",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowMode {
    #[default]
    Dashboard,
    Orb,
}

pub const DEFAULT_ORB_SIZE: u16 = 176;
pub const MIN_ORB_SIZE: u16 = 96;
pub const MAX_ORB_SIZE: u16 = 288;
const ORB_RESIZE_STEP: u16 = 32;

impl WindowMode {
    pub const fn logical_size(self) -> (u16, u16) {
        match self {
            Self::Dashboard => (1120, 680),
            Self::Orb => (DEFAULT_ORB_SIZE, DEFAULT_ORB_SIZE),
        }
    }
}

pub const fn anchored_orb_position(
    dashboard_x: i32,
    dashboard_y: i32,
    dashboard_width: i32,
    orb_width: i32,
) -> (i32, i32) {
    (
        dashboard_x
            .saturating_add(dashboard_width)
            .saturating_sub(orb_width),
        dashboard_y.saturating_add(24),
    )
}

pub const fn centered_resize_position(
    current_x: i32,
    current_y: i32,
    current_size: i32,
    new_size: i32,
) -> (i32, i32) {
    let center_offset = new_size.saturating_sub(current_size) / 2;
    (
        current_x.saturating_sub(center_offset),
        current_y.saturating_sub(center_offset),
    )
}

pub const fn translated_orb_position(
    current_x: i32,
    current_y: i32,
    delta_x: i32,
    delta_y: i32,
) -> (i32, i32) {
    (
        current_x.saturating_add(delta_x),
        current_y.saturating_add(delta_y),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrbResizeAction {
    Smaller,
    Larger,
}

impl OrbResizeAction {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "smaller" => Some(Self::Smaller),
            "larger" => Some(Self::Larger),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WorkspaceSection {
    #[default]
    Today,
    Memory,
    Routines,
}

impl WorkspaceSection {
    pub const ALL: [Self; 3] = [Self::Today, Self::Memory, Self::Routines];

    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Memory => "memory",
            Self::Routines => "routines",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "today" => Some(Self::Today),
            "memory" => Some(Self::Memory),
            "routines" => Some(Self::Routines),
            _ => None,
        }
    }

    pub const fn visibility(self) -> (bool, bool, bool) {
        (
            matches!(self, Self::Today),
            matches!(self, Self::Memory),
            matches!(self, Self::Routines),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OrbState {
    #[default]
    Idle,
    Listening,
    Thinking,
    Working,
    Success,
    Warning,
    Error,
    Sleeping,
}

impl OrbState {
    pub const ALL: [Self; 8] = [
        Self::Idle,
        Self::Listening,
        Self::Thinking,
        Self::Working,
        Self::Success,
        Self::Warning,
        Self::Error,
        Self::Sleeping,
    ];

    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Listening => "listening",
            Self::Thinking => "thinking",
            Self::Working => "working",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Sleeping => "sleeping",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AiProviderChoice {
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

impl AiProviderChoice {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
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

    const fn is_cloud(self) -> bool {
        matches!(
            self,
            Self::OpenAi | Self::Anthropic | Self::Gemini | Self::CustomCloud
        )
    }
}

#[derive(Eq, PartialEq)]
pub struct AiSettingsSubmission {
    pub provider: AiProviderChoice,
    pub new_api_key: Option<String>,
    pub endpoint: String,
    pub model_path: String,
}

impl std::fmt::Debug for AiSettingsSubmission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let api_key = self.new_api_key.as_ref().map(|_| "[REDACTED]");
        formatter
            .debug_struct("AiSettingsSubmission")
            .field("provider", &self.provider)
            .field("new_api_key", &api_key)
            .field("endpoint", &self.endpoint)
            .field("model_path", &self.model_path)
            .finish()
    }
}

impl AiSettingsSubmission {
    pub fn try_from_ui(
        provider: &str,
        api_key: &str,
        endpoint: &str,
        model_path: &str,
        has_existing_key: bool,
    ) -> Result<Self, &'static str> {
        let provider = AiProviderChoice::parse(provider).ok_or("Proveedor de IA no reconocido")?;
        if provider == AiProviderChoice::Disabled {
            return Ok(Self {
                provider,
                new_api_key: None,
                endpoint: String::new(),
                model_path: String::new(),
            });
        }

        let new_api_key = (!api_key.trim().is_empty()).then(|| api_key.to_owned());
        let endpoint = endpoint.trim().to_owned();
        let model_path = model_path.trim().to_owned();

        if provider.is_cloud() && new_api_key.is_none() && !has_existing_key {
            return Err("Agrega una API key para el proveedor cloud");
        }
        if provider == AiProviderChoice::CustomCloud && endpoint.is_empty() {
            return Err("Configura el endpoint del proveedor");
        }
        if provider == AiProviderChoice::CustomLocal && endpoint.is_empty() && model_path.is_empty()
        {
            return Err("Configura un endpoint o modelo local");
        }

        Ok(Self {
            provider,
            new_api_key,
            endpoint,
            model_path,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct NoteSubmission {
    pub title: String,
    pub body: String,
}

impl NoteSubmission {
    pub fn try_from_ui(title: &str, body: &str) -> Result<Self, &'static str> {
        let body = body.trim();
        if body.is_empty() {
            return Err("Escribe algo antes de guardar");
        }

        let title = match title.trim() {
            "" => "Nota rapida",
            value => value,
        };

        Ok(Self {
            title: title.to_owned(),
            body: body.to_owned(),
        })
    }
}

#[derive(Debug)]
pub struct Controller {
    persona: PersonaChoice,
    status_text: &'static str,
    note_count: i32,
    onboarding_visible: bool,
    window_mode: WindowMode,
    orb_state: OrbState,
    active_section: WorkspaceSection,
    orb_size: u16,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            persona: PersonaChoice::Tom,
            status_text: "Listo para ayudarte",
            note_count: 0,
            onboarding_visible: true,
            window_mode: WindowMode::Dashboard,
            orb_state: OrbState::Idle,
            active_section: WorkspaceSection::Today,
            orb_size: DEFAULT_ORB_SIZE,
        }
    }
}

impl Controller {
    pub fn restored(persona: PersonaChoice, note_count: usize) -> Self {
        Self {
            persona,
            status_text: "Memoria local sincronizada",
            note_count: i32::try_from(note_count).unwrap_or(i32::MAX),
            onboarding_visible: false,
            window_mode: WindowMode::Dashboard,
            orb_state: OrbState::Idle,
            active_section: WorkspaceSection::Today,
            orb_size: DEFAULT_ORB_SIZE,
        }
    }

    pub const fn assistant_name(&self) -> &'static str {
        self.persona.display_name()
    }

    pub const fn status_text(&self) -> &'static str {
        self.status_text
    }

    pub const fn note_count(&self) -> i32 {
        self.note_count
    }

    pub const fn onboarding_visible(&self) -> bool {
        self.onboarding_visible
    }

    pub const fn window_mode(&self) -> WindowMode {
        self.window_mode
    }

    pub const fn orb_state(&self) -> OrbState {
        self.orb_state
    }

    pub const fn active_section(&self) -> WorkspaceSection {
        self.active_section
    }

    pub fn navigate_to(&mut self, section: WorkspaceSection) {
        self.active_section = section;
    }

    pub const fn orb_size(&self) -> u16 {
        self.orb_size
    }

    pub fn resize_orb(&mut self, action: OrbResizeAction) {
        self.orb_size = match action {
            OrbResizeAction::Smaller => self
                .orb_size
                .saturating_sub(ORB_RESIZE_STEP)
                .max(MIN_ORB_SIZE),
            OrbResizeAction::Larger => self
                .orb_size
                .saturating_add(ORB_RESIZE_STEP)
                .min(MAX_ORB_SIZE),
        };
    }

    pub fn enter_orb_mode(&mut self) {
        self.window_mode = WindowMode::Orb;
        self.orb_state = OrbState::Idle;
    }

    pub fn restore_dashboard(&mut self) {
        self.window_mode = WindowMode::Dashboard;
    }

    pub fn select_persona(&mut self, persona: PersonaChoice) {
        self.persona = persona;
        self.onboarding_visible = false;
        self.status_text = match persona {
            PersonaChoice::Tom => "Modo profesional activado",
            PersonaChoice::Tomy => "Modo cercano activado",
        };
        self.orb_state = OrbState::Success;
    }

    pub fn record_note_saved(&mut self) {
        self.note_count = self.note_count.saturating_add(1);
        self.status_text = "Nota guardada en tu memoria local";
        self.orb_state = OrbState::Success;
    }

    pub fn record_invalid_note(&mut self) {
        self.status_text = "Escribe algo antes de guardar";
        self.orb_state = OrbState::Warning;
    }

    pub fn record_routine_accepted(&mut self) {
        self.status_text = "Rutina activada";
        self.orb_state = OrbState::Success;
    }

    pub fn record_routine_dismissed(&mut self) {
        self.status_text = "Sugerencia descartada";
        self.orb_state = OrbState::Idle;
    }

    pub fn record_storage_error(&mut self) {
        self.status_text = "No pudimos acceder a la memoria local";
        self.orb_state = OrbState::Error;
    }

    pub fn record_ai_settings_saved(&mut self) {
        self.status_text = "Configuracion de IA guardada";
        self.orb_state = OrbState::Success;
    }

    pub fn record_ai_validation_error(&mut self, message: &'static str) {
        self.status_text = message;
        self.orb_state = OrbState::Warning;
    }

    pub fn record_credential_error(&mut self) {
        self.status_text = "No pudimos acceder al almacen seguro de Windows";
        self.orb_state = OrbState::Error;
    }

    pub fn record_api_key_cleared(&mut self) {
        self.status_text = "API key eliminada de Windows";
        self.orb_state = OrbState::Success;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AiProviderChoice, AiSettingsSubmission, Controller, NoteSubmission, OrbResizeAction,
        OrbState, PersonaChoice, WindowMode, WorkspaceSection, anchored_orb_position,
        centered_resize_position, translated_orb_position,
    };

    #[test]
    fn workspace_navigation_starts_today_and_switches_sections() {
        let mut controller = Controller::default();

        assert_eq!(controller.active_section(), WorkspaceSection::Today);

        controller.navigate_to(WorkspaceSection::Memory);
        assert_eq!(controller.active_section(), WorkspaceSection::Memory);

        controller.navigate_to(WorkspaceSection::Routines);
        assert_eq!(controller.active_section(), WorkspaceSection::Routines);
    }

    #[test]
    fn workspace_section_ids_round_trip() {
        for section in WorkspaceSection::ALL {
            assert_eq!(WorkspaceSection::parse(section.stable_id()), Some(section));
        }
        assert_eq!(WorkspaceSection::parse("unknown"), None);
    }

    #[test]
    fn workspace_sections_expose_one_visible_page() {
        assert_eq!(WorkspaceSection::Today.visibility(), (true, false, false));
        assert_eq!(WorkspaceSection::Memory.visibility(), (false, true, false));
        assert_eq!(
            WorkspaceSection::Routines.visibility(),
            (false, false, true)
        );
    }

    #[test]
    fn orb_resizes_in_bounded_steps() {
        let mut controller = Controller::default();

        assert_eq!(controller.orb_size(), 176);
        controller.resize_orb(OrbResizeAction::Smaller);
        assert_eq!(controller.orb_size(), 144);

        for _ in 0..10 {
            controller.resize_orb(OrbResizeAction::Smaller);
        }
        assert_eq!(controller.orb_size(), 96);

        for _ in 0..10 {
            controller.resize_orb(OrbResizeAction::Larger);
        }
        assert_eq!(controller.orb_size(), 288);
    }

    #[test]
    fn orb_resize_actions_parse_ui_values() {
        assert_eq!(
            OrbResizeAction::parse("smaller"),
            Some(OrbResizeAction::Smaller)
        );
        assert_eq!(
            OrbResizeAction::parse("larger"),
            Some(OrbResizeAction::Larger)
        );
        assert_eq!(OrbResizeAction::parse("unknown"), None);
    }

    #[test]
    fn resizing_an_orb_preserves_its_screen_center() {
        assert_eq!(centered_resize_position(100, 80, 176, 208), (84, 64));
        assert_eq!(centered_resize_position(-20, -10, 96, 96), (-20, -10));
    }

    #[test]
    fn dragging_an_orb_translates_its_screen_position() {
        assert_eq!(translated_orb_position(100, 80, 15, -6), (115, 74));
        assert_eq!(
            translated_orb_position(i32::MAX, i32::MIN, 20, -20),
            (i32::MAX, i32::MIN)
        );
    }

    #[test]
    fn orb_is_anchored_to_the_dashboard_top_right_without_changing_shape() {
        let placement = anchored_orb_position(100, 80, 1120, 176);

        assert_eq!(placement, (1044, 104));
        let (width, height) = WindowMode::Orb.logical_size();
        assert_eq!(width, height);
    }

    #[test]
    fn persona_choice_accepts_ui_values_case_insensitively() {
        assert_eq!(PersonaChoice::parse("TOMY"), PersonaChoice::Tomy);
        assert_eq!(PersonaChoice::parse("friendly"), PersonaChoice::Tomy);
        assert_eq!(PersonaChoice::parse("PROFESSIONAL"), PersonaChoice::Tom);
        assert_eq!(PersonaChoice::parse("unexpected"), PersonaChoice::Tom);
    }

    #[test]
    fn selecting_a_persona_finishes_onboarding() {
        let mut controller = Controller::default();

        controller.select_persona(PersonaChoice::Tomy);

        assert_eq!(controller.assistant_name(), "Tomy");
        assert!(!controller.onboarding_visible());
        assert_eq!(controller.status_text(), "Modo cercano activado");
    }

    #[test]
    fn window_mode_exposes_dashboard_and_orb_sizes() {
        assert_eq!(WindowMode::Dashboard.logical_size(), (1120, 680));
        assert_eq!(WindowMode::Orb.logical_size(), (176, 176));

        let mut controller = Controller::default();
        controller.enter_orb_mode();
        assert_eq!(controller.window_mode(), WindowMode::Orb);

        controller.restore_dashboard();
        assert_eq!(controller.window_mode(), WindowMode::Dashboard);
    }

    #[test]
    fn user_feedback_selects_a_matching_orb_state() {
        let mut controller = Controller::default();
        assert_eq!(controller.orb_state(), OrbState::Idle);

        controller.record_note_saved();
        assert_eq!(controller.orb_state(), OrbState::Success);

        controller.record_invalid_note();
        assert_eq!(controller.orb_state(), OrbState::Warning);

        controller.record_storage_error();
        assert_eq!(controller.orb_state(), OrbState::Error);
    }

    #[test]
    fn every_orb_visual_state_has_a_stable_asset_id() {
        let state_ids = OrbState::ALL.map(OrbState::stable_id);

        assert_eq!(
            state_ids,
            [
                "idle",
                "listening",
                "thinking",
                "working",
                "success",
                "warning",
                "error",
                "sleeping",
            ]
        );
    }

    #[test]
    fn a_note_requires_content_and_uses_a_fallback_title() {
        assert_eq!(
            NoteSubmission::try_from_ui("", "  Remember the demo  "),
            Ok(NoteSubmission {
                title: "Nota rapida".into(),
                body: "Remember the demo".into(),
            })
        );
        assert_eq!(
            NoteSubmission::try_from_ui("ignored", "   "),
            Err("Escribe algo antes de guardar")
        );
    }

    #[test]
    fn invalid_note_feedback_is_visible_without_changing_the_count() {
        let mut controller = Controller::default();

        controller.record_invalid_note();

        assert_eq!(controller.note_count(), 0);
        assert_eq!(controller.status_text(), "Escribe algo antes de guardar");
    }

    #[test]
    fn recording_a_saved_note_updates_count_and_status() {
        let mut controller = Controller::default();

        controller.record_note_saved();

        assert_eq!(controller.note_count(), 1);
        assert_eq!(
            controller.status_text(),
            "Nota guardada en tu memoria local"
        );
    }

    #[test]
    fn routine_decisions_surface_immediate_feedback() {
        let mut controller = Controller::default();

        controller.record_routine_accepted();
        assert_eq!(controller.status_text(), "Rutina activada");

        controller.record_routine_dismissed();
        assert_eq!(controller.status_text(), "Sugerencia descartada");
    }

    #[test]
    fn storage_errors_are_safe_to_show_in_the_interface() {
        let mut controller = Controller::default();

        controller.record_storage_error();

        assert_eq!(
            controller.status_text(),
            "No pudimos acceder a la memoria local"
        );
    }

    #[test]
    fn restored_state_skips_onboarding_and_caps_large_note_counts() {
        let controller = Controller::restored(PersonaChoice::Tomy, usize::MAX);

        assert_eq!(controller.assistant_name(), "Tomy");
        assert!(!controller.onboarding_visible());
        assert_eq!(controller.note_count(), i32::MAX);
    }

    #[test]
    fn ai_provider_ids_round_trip_and_unknown_values_are_rejected() {
        let providers = [
            AiProviderChoice::Disabled,
            AiProviderChoice::OpenAi,
            AiProviderChoice::Anthropic,
            AiProviderChoice::Gemini,
            AiProviderChoice::CustomCloud,
            AiProviderChoice::Ollama,
            AiProviderChoice::LmStudio,
            AiProviderChoice::CustomLocal,
        ];

        for provider in providers {
            assert_eq!(
                AiProviderChoice::parse(provider.stable_id()),
                Some(provider)
            );
        }
        assert_eq!(AiProviderChoice::parse("unknown"), None);
    }

    #[test]
    fn empty_api_key_preserves_an_existing_credential() {
        let submission = AiSettingsSubmission::try_from_ui(
            "openai",
            "   ",
            " https://api.openai.com/v1 ",
            " ",
            true,
        )
        .expect("existing key should satisfy cloud provider");

        assert_eq!(submission.provider, AiProviderChoice::OpenAi);
        assert_eq!(submission.new_api_key, None);
        assert_eq!(submission.endpoint, "https://api.openai.com/v1");
        assert_eq!(submission.model_path, "");
    }

    #[test]
    fn ai_submission_debug_output_redacts_the_api_key() {
        let submission = AiSettingsSubmission::try_from_ui("openai", "super-secret", "", "", false)
            .expect("submission should be valid");

        let debug = format!("{submission:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn cloud_provider_requires_a_new_or_existing_api_key() {
        assert_eq!(
            AiSettingsSubmission::try_from_ui("anthropic", "", "", "", false),
            Err("Agrega una API key para el proveedor cloud")
        );

        let submission = AiSettingsSubmission::try_from_ui("gemini", "secret", "", "", false)
            .expect("new key should satisfy cloud provider");
        assert_eq!(submission.new_api_key.as_deref(), Some("secret"));
    }

    #[test]
    fn custom_providers_require_a_location() {
        assert_eq!(
            AiSettingsSubmission::try_from_ui("custom-cloud", "secret", "", "", false),
            Err("Configura el endpoint del proveedor")
        );
        assert_eq!(
            AiSettingsSubmission::try_from_ui("custom-local", "", "", "", false),
            Err("Configura un endpoint o modelo local")
        );
    }

    #[test]
    fn disabled_provider_discards_non_secret_locations() {
        let submission = AiSettingsSubmission::try_from_ui(
            "disabled",
            "",
            "http://unused",
            "unused.gguf",
            false,
        )
        .expect("disabled is always valid");

        assert_eq!(submission.endpoint, "");
        assert_eq!(submission.model_path, "");
        assert_eq!(submission.new_api_key, None);
    }

    #[test]
    fn ai_feedback_never_contains_credential_details() {
        let mut controller = Controller::default();

        controller.record_ai_validation_error("Configura el endpoint del proveedor");
        assert_eq!(
            controller.status_text(),
            "Configura el endpoint del proveedor"
        );

        controller.record_ai_settings_saved();
        assert_eq!(controller.status_text(), "Configuracion de IA guardada");

        controller.record_credential_error();
        assert_eq!(
            controller.status_text(),
            "No pudimos acceder al almacen seguro de Windows"
        );

        controller.record_api_key_cleared();
        assert_eq!(controller.status_text(), "API key eliminada de Windows");
    }
}
