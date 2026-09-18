use tom_core::{
    AiProvider, AiSetup, AssistanceStyle, OnboardingStep, Persona, ProfessionalProfile, UserProfile,
};

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
        Self::ALL
            .into_iter()
            .find(|section| section.stable_id() == value)
    }

    pub const fn visibility(self) -> (bool, bool, bool) {
        (
            matches!(self, Self::Today),
            matches!(self, Self::Memory),
            matches!(self, Self::Routines),
        )
    }

    /// Heading shown in the workspace header.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Today => "Espacio de enfoque",
            Self::Memory => "Archivo de memoria",
            Self::Routines => "Rutinas explicables",
        }
    }

    /// Supporting line under the heading, spoken about the assistant by name.
    pub fn subtitle(self, assistant_name: &str) -> String {
        match self {
            Self::Today => format!("Captura lo importante. {assistant_name} mantiene el hilo."),
            Self::Memory => "Decisiones y contexto que tú controlas.".to_owned(),
            Self::Routines => "Revisa la evidencia antes de activar cualquier ayuda.".to_owned(),
        }
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
    Curious,
}

impl OrbState {
    pub const ALL: [Self; 9] = [
        Self::Idle,
        Self::Listening,
        Self::Thinking,
        Self::Working,
        Self::Success,
        Self::Warning,
        Self::Error,
        Self::Sleeping,
        Self::Curious,
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
            Self::Curious => "curious",
        }
    }

    /// Resolves an id back into a state; every id has a matching orb asset.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|state| state.stable_id() == value)
    }
}

/// A connection the user asked to save. Credentials never travel through this form: they
/// are managed on their own, so nothing here is secret.
#[derive(Debug, Eq, PartialEq)]
pub struct AiSettingsSubmission {
    pub provider: AiProvider,
    pub endpoint: String,
    pub model_path: String,
}

impl AiSettingsSubmission {
    /// Credentials are not checked here. No key is required to save a connection: Tom only
    /// needs one way in overall, and whether it has one is asked of the vault, not the form.
    pub fn try_from_ui(
        provider: &str,
        endpoint: &str,
        model_path: &str,
    ) -> Result<Self, &'static str> {
        let provider = AiProvider::from_stable_id(provider).ok_or(UNKNOWN_PROVIDER)?;
        if provider == AiProvider::Disabled {
            return Ok(Self {
                provider,
                endpoint: String::new(),
                model_path: String::new(),
            });
        }

        let endpoint = endpoint.trim().to_owned();
        let model_path = model_path.trim().to_owned();

        if provider.requires_endpoint() && endpoint.is_empty() {
            return Err("Configura el endpoint del proveedor");
        }
        if provider.requires_model_location() && endpoint.is_empty() && model_path.is_empty() {
            return Err("Configura un endpoint o modelo local");
        }

        Ok(Self {
            provider,
            endpoint,
            model_path,
        })
    }
}

/// A credential the user asked to store for one provider.
#[derive(Eq, PartialEq)]
pub struct ApiKeySubmission {
    pub provider: AiProvider,
    pub api_key: String,
}

impl std::fmt::Debug for ApiKeySubmission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApiKeySubmission")
            .field("provider", &self.provider)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

impl ApiKeySubmission {
    /// Surrounding whitespace is dropped, since it only ever arrives from a sloppy paste and
    /// no provider issues keys that begin or end with it. The key itself is never altered.
    pub fn try_from_ui(provider: &str, api_key: &str) -> Result<Self, &'static str> {
        let provider = AiProvider::from_stable_id(provider).ok_or(UNKNOWN_PROVIDER)?;
        if !provider.accepts_api_key() {
            return Err(PROVIDER_WITHOUT_KEY);
        }
        if api_key.trim().is_empty() {
            return Err(EMPTY_API_KEY);
        }

        Ok(Self {
            provider,
            api_key: api_key.trim().to_owned(),
        })
    }
}

pub const UNKNOWN_PROVIDER: &str = "Proveedor de IA no reconocido";
pub const UNKNOWN_PROFILE: &str = "Perfil profesional no reconocido";
pub const UNKNOWN_ASSISTANCE: &str = "Nivel de iniciativa no reconocido";
pub const UNKNOWN_PERSONA: &str = "Personalidad no reconocida";
pub const UNKNOWN_AI_SETUP: &str = "Opcion de inteligencia no reconocida";
pub const EMPTY_API_KEY: &str = "Pega una clave antes de guardarla";
pub const PROVIDER_WITHOUT_KEY: &str = "Este proveedor no usa claves";
/// Shown whenever Tom has neither a stored key nor a local model it can reach.
pub const NO_INTELLIGENCE: &str = "Tom no puede pensar: anade una clave o un modelo local";

/// The four answers onboarding collects, resolved from the ids the interface sends back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OnboardingAnswers {
    pub persona: Persona,
    pub professional_profile: ProfessionalProfile,
    pub assistance_style: AssistanceStyle,
    pub ai_setup: AiSetup,
}

impl OnboardingAnswers {
    /// Rejects the whole set when any id is unknown, so a half-parsed profile is never saved.
    pub fn try_from_ui(
        persona: &str,
        professional_profile: &str,
        assistance_style: &str,
        ai_setup: &str,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            persona: Persona::from_stable_id(persona).ok_or(UNKNOWN_PERSONA)?,
            professional_profile: ProfessionalProfile::from_stable_id(professional_profile)
                .ok_or(UNKNOWN_PROFILE)?,
            assistance_style: AssistanceStyle::from_stable_id(assistance_style)
                .ok_or(UNKNOWN_ASSISTANCE)?,
            ai_setup: AiSetup::from_stable_id(ai_setup).ok_or(UNKNOWN_AI_SETUP)?,
        })
    }

    pub const fn profile(self) -> UserProfile {
        UserProfile {
            persona: self.persona,
            professional_profile: self.professional_profile,
            assistance_style: self.assistance_style,
        }
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

/// What pressing the onboarding primary button did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OnboardingAdvance {
    /// The flow moved to the next question.
    Moved,
    /// The last question was answered and the profile is ready to be saved.
    Finished,
}

#[derive(Debug)]
pub struct Controller {
    persona: Persona,
    status_text: &'static str,
    note_count: i32,
    onboarding_visible: bool,
    onboarding_step: OnboardingStep,
    window_mode: WindowMode,
    orb_state: OrbState,
    active_section: WorkspaceSection,
    orb_size: u16,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            persona: Persona::Tom,
            status_text: "Listo para ayudarte",
            note_count: 0,
            onboarding_visible: true,
            onboarding_step: OnboardingStep::Welcome,
            window_mode: WindowMode::Dashboard,
            orb_state: OrbState::Idle,
            active_section: WorkspaceSection::Today,
            orb_size: DEFAULT_ORB_SIZE,
        }
    }
}

impl Controller {
    pub fn restored(persona: Persona, note_count: usize) -> Self {
        let mut controller = Self {
            persona,
            status_text: "Memoria local sincronizada",
            onboarding_visible: false,
            ..Self::default()
        };
        controller.set_note_count(note_count);
        controller
    }

    pub const fn persona(&self) -> Persona {
        self.persona
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

    /// Adopts the number of notes the store actually holds.
    ///
    /// The counter is a view of persisted state, not a tally of successful saves, so every
    /// reload re-reads it instead of trusting an increment that a failed write could skew.
    pub fn set_note_count(&mut self, note_count: usize) {
        self.note_count = i32::try_from(note_count).unwrap_or(i32::MAX);
    }

    pub const fn onboarding_visible(&self) -> bool {
        self.onboarding_visible
    }

    /// Index of the current question, for the progress rail the interface draws.
    pub fn onboarding_step_index(&self) -> i32 {
        i32::try_from(self.onboarding_step.index()).unwrap_or_default()
    }

    pub fn onboarding_can_go_back(&self) -> bool {
        self.onboarding_step.previous().is_some()
    }

    /// Moves to the next question, or reports that the flow is ready to finish.
    pub fn advance_onboarding(&mut self) -> OnboardingAdvance {
        match self.onboarding_step.next() {
            Some(step) => {
                self.onboarding_step = step;
                OnboardingAdvance::Moved
            }
            None => OnboardingAdvance::Finished,
        }
    }

    /// Returns to the previous question; the first question has nowhere to go.
    pub fn back_onboarding(&mut self) {
        if let Some(step) = self.onboarding_step.previous() {
            self.onboarding_step = step;
        }
    }

    /// The orb face onboarding asks for, falling back to the assistant's live state.
    pub fn onboarding_orb_state(&self) -> OrbState {
        self.onboarding_step
            .orb_state_id()
            .and_then(OrbState::parse)
            .unwrap_or(self.orb_state)
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

    /// Header copy for the section currently on screen.
    pub fn section_heading(&self) -> (&'static str, String) {
        (
            self.active_section.title(),
            self.active_section.subtitle(self.assistant_name()),
        )
    }

    /// Shown wherever the routine list is empty, on Today and on Routines alike.
    pub fn empty_routines_text(&self) -> String {
        format!(
            "Sin propuestas pendientes. {} seguirá observando sólo las señales que autorices.",
            self.assistant_name()
        )
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

    /// Reopens onboarding from the first question without discarding earlier answers.
    pub fn replay_onboarding(&mut self) {
        self.onboarding_visible = true;
        self.onboarding_step = OnboardingStep::Welcome;
        self.status_text = "Repasemos tu configuracion";
        self.orb_state = OrbState::Curious;
    }

    /// Adopts a persona while onboarding is still open, so the flow speaks in the chosen voice.
    pub fn preview_persona(&mut self, persona: Persona) {
        self.persona = persona;
    }

    pub fn select_persona(&mut self, persona: Persona) {
        self.persona = persona;
        self.onboarding_visible = false;
        self.onboarding_step = OnboardingStep::Welcome;
        self.status_text = match persona {
            Persona::Tom => "Modo profesional activado",
            Persona::Tomy => "Modo cercano activado",
        };
        self.orb_state = OrbState::Success;
    }

    pub fn record_note_saved(&mut self) {
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

    /// Reports the one configuration Tom cannot work with: no key and no local model.
    pub fn record_missing_intelligence(&mut self) {
        self.status_text = NO_INTELLIGENCE;
        self.orb_state = OrbState::Warning;
    }

    pub fn record_api_key_stored(&mut self) {
        self.status_text = "Clave guardada en Windows";
        self.orb_state = OrbState::Success;
    }

    pub fn record_api_key_cleared(&mut self) {
        self.status_text = "Clave eliminada de Windows";
        self.orb_state = OrbState::Success;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AiSettingsSubmission, ApiKeySubmission, Controller, NoteSubmission, OnboardingAdvance,
        OnboardingAnswers, OrbResizeAction, OrbState, WindowMode, WorkspaceSection,
        anchored_orb_position, centered_resize_position, translated_orb_position,
    };
    use tom_core::{
        AiProvider, AiSettings, AiSetup, AssistanceStyle, OnboardingStep, Persona,
        ProfessionalProfile,
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
    fn section_headings_name_the_active_persona_only_where_it_belongs() {
        let mut controller = Controller::default();

        let (title, subtitle) = controller.section_heading();
        assert_eq!(title, "Espacio de enfoque");
        assert!(subtitle.contains("Tom"));

        controller.select_persona(Persona::Tomy);
        assert!(controller.section_heading().1.contains("Tomy"));
        assert!(controller.empty_routines_text().contains("Tomy"));

        controller.navigate_to(WorkspaceSection::Memory);
        let (title, subtitle) = controller.section_heading();
        assert_eq!(title, "Archivo de memoria");
        assert!(!subtitle.contains("Tomy"));
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
    fn selecting_a_persona_finishes_onboarding() {
        let mut controller = Controller::default();

        controller.select_persona(Persona::Tomy);

        assert_eq!(controller.assistant_name(), "Tomy");
        assert_eq!(controller.persona(), Persona::Tomy);
        assert!(!controller.onboarding_visible());
        assert_eq!(controller.status_text(), "Modo cercano activado");
    }

    #[test]
    fn replaying_onboarding_restarts_the_flow_but_keeps_the_chosen_persona() {
        let mut controller = Controller::default();
        controller.advance_onboarding();
        controller.select_persona(Persona::Tomy);

        controller.replay_onboarding();

        assert!(controller.onboarding_visible());
        assert_eq!(controller.onboarding_step_index(), 0);
        assert_eq!(controller.persona(), Persona::Tomy);
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
                "curious",
            ]
        );
        for state in OrbState::ALL {
            assert_eq!(OrbState::parse(state.stable_id()), Some(state));
        }
        assert_eq!(OrbState::parse("unknown"), None);
    }

    #[test]
    fn onboarding_orb_hints_name_real_orb_states() {
        for step in OnboardingStep::ALL {
            if let Some(id) = step.orb_state_id() {
                assert!(OrbState::parse(id).is_some(), "{id}");
            }
        }

        let mut controller = Controller::default();
        assert_eq!(controller.onboarding_orb_state(), OrbState::Curious);

        controller.record_storage_error();
        controller.advance_onboarding();
        assert_eq!(controller.onboarding_orb_state(), OrbState::Error);
    }

    #[test]
    fn onboarding_walks_forward_then_asks_to_finish() {
        let mut controller = Controller::default();

        assert!(!controller.onboarding_can_go_back());
        assert_eq!(controller.onboarding_step_index(), 0);

        for expected in 1..i32::try_from(OnboardingStep::ALL.len()).expect("five steps fit") {
            assert_eq!(controller.advance_onboarding(), OnboardingAdvance::Moved);
            assert_eq!(controller.onboarding_step_index(), expected);
            assert!(controller.onboarding_can_go_back());
        }

        let last = i32::try_from(OnboardingStep::LAST.index()).expect("five steps fit");
        assert_eq!(controller.onboarding_step_index(), last);
        assert_eq!(controller.advance_onboarding(), OnboardingAdvance::Finished);
        assert_eq!(controller.onboarding_step_index(), last);
    }

    #[test]
    fn onboarding_answers_resolve_every_id_or_none_of_them() {
        let answers = OnboardingAnswers::try_from_ui("tomy", "creator", "proactive", "local")
            .expect("all four ids are known");

        assert_eq!(answers.persona, Persona::Tomy);
        assert_eq!(answers.professional_profile, ProfessionalProfile::Creator);
        assert_eq!(answers.assistance_style, AssistanceStyle::Proactive);
        assert_eq!(answers.ai_setup, AiSetup::Local);

        let profile = answers.profile();
        assert_eq!(profile.persona, Persona::Tomy);
        assert_eq!(profile.assistance_style, AssistanceStyle::Proactive);
    }

    #[test]
    fn an_unknown_onboarding_id_is_reported_instead_of_defaulted() {
        assert_eq!(
            OnboardingAnswers::try_from_ui("friendly", "creator", "proactive", "local"),
            Err(super::UNKNOWN_PERSONA)
        );
        assert_eq!(
            OnboardingAnswers::try_from_ui("tom", "gardener", "proactive", "local"),
            Err(super::UNKNOWN_PROFILE)
        );
        assert_eq!(
            OnboardingAnswers::try_from_ui("tom", "creator", "eager", "local"),
            Err(super::UNKNOWN_ASSISTANCE)
        );
        assert_eq!(
            OnboardingAnswers::try_from_ui("tom", "creator", "proactive", "someday"),
            Err(super::UNKNOWN_AI_SETUP)
        );
    }

    #[test]
    fn previewing_a_persona_changes_the_voice_without_closing_onboarding() {
        let mut controller = Controller::default();

        controller.preview_persona(Persona::Tomy);

        assert_eq!(controller.assistant_name(), "Tomy");
        assert!(controller.onboarding_visible());
    }

    #[test]
    fn going_back_stops_at_the_first_question() {
        let mut controller = Controller::default();

        controller.back_onboarding();
        assert_eq!(controller.onboarding_step_index(), 0);

        controller.advance_onboarding();
        controller.back_onboarding();
        assert_eq!(controller.onboarding_step_index(), 0);
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
    fn the_note_counter_mirrors_the_store_rather_than_the_saves() {
        let mut controller = Controller::default();

        controller.record_note_saved();
        controller.set_note_count(7);

        assert_eq!(controller.note_count(), 7);
        assert_eq!(
            controller.status_text(),
            "Nota guardada en tu memoria local"
        );

        controller.set_note_count(usize::MAX);
        assert_eq!(controller.note_count(), i32::MAX);
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
        let controller = Controller::restored(Persona::Tomy, usize::MAX);

        assert_eq!(controller.assistant_name(), "Tomy");
        assert!(!controller.onboarding_visible());
        assert_eq!(controller.note_count(), i32::MAX);
    }

    #[test]
    fn saving_a_connection_never_demands_a_credential() {
        let submission =
            AiSettingsSubmission::try_from_ui("openai", " https://api.openai.com/v1 ", " ")
                .expect("no key is required to save a connection");

        assert_eq!(submission.provider, AiProvider::OpenAi);
        assert_eq!(submission.endpoint, "https://api.openai.com/v1");
        assert_eq!(submission.model_path, "");

        AiSettingsSubmission::try_from_ui("anthropic", "", "")
            .expect("a cloud provider saves fine with no key stored yet");
        AiSettingsSubmission::try_from_ui("ollama", "", "")
            .expect("a local provider saves fine too");
    }

    #[test]
    fn custom_providers_still_need_somewhere_to_connect() {
        assert_eq!(
            AiSettingsSubmission::try_from_ui("custom-cloud", "", ""),
            Err("Configura el endpoint del proveedor")
        );
        assert_eq!(
            AiSettingsSubmission::try_from_ui("custom-local", "", ""),
            Err("Configura un endpoint o modelo local")
        );
    }

    #[test]
    fn an_unknown_provider_is_rejected_instead_of_falling_back() {
        assert_eq!(
            AiSettingsSubmission::try_from_ui("gpt-9", "", ""),
            Err(super::UNKNOWN_PROVIDER)
        );
    }

    #[test]
    fn disabled_provider_discards_non_secret_locations() {
        let submission =
            AiSettingsSubmission::try_from_ui("disabled", "http://unused", "unused.gguf")
                .expect("disabled is always valid");

        assert_eq!(submission.endpoint, "");
        assert_eq!(submission.model_path, "");
    }

    #[test]
    fn one_way_in_is_enough_and_none_is_what_stops_tom() {
        let nothing = AiSettings::default();
        assert!(!nothing.intelligence_is_available(0));

        // A single key, from any provider, is enough on its own.
        assert!(nothing.intelligence_is_available(1));

        // So is a local model, with no key stored anywhere.
        let local = AiSettings {
            provider: AiProvider::Ollama,
            endpoint: "http://127.0.0.1:11434".to_owned(),
            model_path: String::new(),
        };
        assert!(local.has_reachable_local_model());
        assert!(local.intelligence_is_available(0));

        // A local provider that names nowhere is not a way in.
        let unreachable = AiSettings {
            provider: AiProvider::Ollama,
            endpoint: "   ".to_owned(),
            model_path: String::new(),
        };
        assert!(!unreachable.has_reachable_local_model());
        assert!(!unreachable.intelligence_is_available(0));

        // A cloud provider is never a way in by itself; its key is.
        let cloud = AiSettings {
            provider: AiProvider::OpenAi,
            endpoint: "https://api.openai.com/v1".to_owned(),
            model_path: String::new(),
        };
        assert!(!cloud.has_reachable_local_model());
        assert!(!cloud.intelligence_is_available(0));
        assert!(cloud.intelligence_is_available(1));
    }

    #[test]
    fn having_no_way_in_is_reported_to_the_user() {
        let mut controller = Controller::default();

        controller.record_missing_intelligence();

        assert_eq!(controller.status_text(), super::NO_INTELLIGENCE);
        assert_eq!(controller.orb_state(), OrbState::Warning);
    }

    #[test]
    fn a_key_submission_keeps_the_pasted_value_and_hides_it_from_logs() {
        let submission = ApiKeySubmission::try_from_ui("anthropic", "  super-secret  ")
            .expect("a pasted key is valid");

        assert_eq!(submission.provider, AiProvider::Anthropic);
        assert_eq!(submission.api_key, "super-secret");

        let debug = format!("{submission:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn a_key_submission_refuses_blanks_and_providers_that_take_no_key() {
        assert_eq!(
            ApiKeySubmission::try_from_ui("openai", "   "),
            Err(super::EMPTY_API_KEY)
        );
        assert_eq!(
            ApiKeySubmission::try_from_ui("disabled", "secret"),
            Err(super::PROVIDER_WITHOUT_KEY)
        );
        assert_eq!(
            ApiKeySubmission::try_from_ui("gpt-9", "secret"),
            Err(super::UNKNOWN_PROVIDER)
        );
    }

    #[test]
    fn a_local_provider_may_still_hold_an_optional_key() {
        let submission = ApiKeySubmission::try_from_ui("lm-studio", "proxy-token")
            .expect("local servers behind a proxy can carry a token");

        assert_eq!(submission.provider, AiProvider::LmStudio);
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

        controller.record_api_key_stored();
        assert_eq!(controller.status_text(), "Clave guardada en Windows");

        controller.record_api_key_cleared();
        assert_eq!(controller.status_text(), "Clave eliminada de Windows");
    }
}
