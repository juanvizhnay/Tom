#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod controller;
mod credential;
mod transport;
mod view_model;

use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

use controller::{
    AiSettingsSubmission, ApiKeySubmission, ChatSubmission, Controller, NO_INTELLIGENCE,
    NoteSubmission, OnboardingAdvance, OnboardingAnswers, OrbResizeAction, OrbState,
    UNKNOWN_PERSONA, UNKNOWN_PROVIDER, WindowMode, WorkspaceSection, anchored_orb_position,
    centered_resize_position, translated_orb_position,
};
use credential::{
    CredentialError, CredentialVault, WindowsCredentialVault, forget_api_key, read_key_inventory,
    store_api_key,
};
use directories::ProjectDirs;
use slint::{
    ComponentHandle, LogicalPosition, LogicalSize, Model, PhysicalPosition, PhysicalSize, VecModel,
};
use tom_core::{
    AiProvider, AiSettings, MemoryStore, Note, Persona, UserProfile, build_chat_request,
    system_prompt,
};
use view_model::{api_key_rows, bind_catalog, bind_orb_bounds};

slint::include_modules!();

/// Everything the interface callbacks need: mutable app state, storage, and the key vault.
#[derive(Clone)]
struct AppContext {
    controller: Rc<RefCell<Controller>>,
    store: Rc<Option<MemoryStore>>,
    vault: Rc<WindowsCredentialVault>,
}

impl AppContext {
    fn store(&self) -> Option<&MemoryStore> {
        self.store.as_ref().as_ref()
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let orb_ui = OrbWindow::new()?;
    let store = Rc::new(open_memory_store());
    let notes = load_notes(store.as_ref().as_ref());
    let mut initial_controller = load_controller(store.as_ref().as_ref(), notes.len());
    let ai_settings = load_ai_settings(store.as_ref().as_ref(), &mut initial_controller);
    let vault = Rc::new(WindowsCredentialVault);
    let has_api_key = provider_has_api_key(*vault, ai_settings.provider).unwrap_or_else(|_| {
        initial_controller.record_credential_error();
        false
    });
    let selected_key_provider = if ai_settings.provider.accepts_api_key() {
        ai_settings.provider
    } else {
        AiProvider::OpenAi
    };
    let context = AppContext {
        controller: Rc::new(RefCell::new(initial_controller)),
        store,
        vault,
    };
    let routine_model = Rc::new(VecModel::from(initial_routines()));

    bind_catalog(&ui);
    bind_orb_bounds(&orb_ui);
    sync_ui(&ui, &context.controller.borrow());
    sync_orb_ui(&orb_ui, &context.controller.borrow());
    sync_profile_ui(&ui, context.store());
    sync_ai_ui(&ui, &ai_settings, has_api_key);
    ui.set_selected_key_provider(selected_key_provider.stable_id().into());
    refresh_key_manager(&ui, &context);
    set_memory_rows(&ui, notes);
    ui.set_routine_rows(Rc::clone(&routine_model).into());

    connect_onboarding_callbacks(&ui, &context);
    connect_navigation_callback(&ui, Rc::clone(&context.controller));
    connect_window_callbacks(&ui, &orb_ui, Rc::clone(&context.controller));
    connect_note_callback(&ui, &context);
    connect_routine_callbacks(&ui, Rc::clone(&context.controller), routine_model);
    connect_ai_callbacks(&ui, &context);
    connect_conversation_callbacks(&ui, &context);

    ui.run()
}

fn open_memory_store() -> Option<MemoryStore> {
    let database_path = database_path()?;
    let parent = database_path.parent()?;
    fs::create_dir_all(parent).ok()?;
    MemoryStore::open(database_path).ok()
}

fn database_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "Tom", "Tom")
        .map(|directories| directories.data_local_dir().join("memory.sqlite3"))
}

fn load_notes(store: Option<&MemoryStore>) -> Vec<Note> {
    store
        .and_then(|store| store.list_notes().ok())
        .unwrap_or_default()
}

fn load_controller(store: Option<&MemoryStore>, note_count: usize) -> Controller {
    let Some(store) = store else {
        let mut controller = Controller::default();
        controller.record_storage_error();
        return controller;
    };

    match store.load_profile() {
        Ok(Some(profile)) => Controller::restored(profile.persona, note_count),
        // Onboarding has not finished, but notes may already exist: the counter reflects the
        // store either way instead of claiming the archive is empty.
        Ok(None) => {
            let mut controller = Controller::default();
            controller.set_note_count(note_count);
            controller
        }
        Err(_) => {
            let mut controller = Controller::default();
            controller.record_storage_error();
            controller
        }
    }
}

fn load_ai_settings(store: Option<&MemoryStore>, controller: &mut Controller) -> AiSettings {
    let Some(store) = store else {
        return AiSettings::default();
    };

    store.load_ai_settings().unwrap_or_else(|_| {
        controller.record_storage_error();
        AiSettings::default()
    })
}

fn sync_ui(ui: &MainWindow, controller: &Controller) {
    debug_assert!(OrbState::ALL.contains(&controller.orb_state()));
    debug_assert!(WorkspaceSection::ALL.contains(&controller.active_section()));
    let (section_title, section_subtitle) = controller.section_heading();
    ui.set_assistant_name(controller.assistant_name().into());
    ui.set_status_text(controller.status_text().into());
    ui.set_note_count(controller.note_count());
    ui.set_onboarding_visible(controller.onboarding_visible());
    ui.set_onboarding_step(controller.onboarding_step_index());
    ui.set_onboarding_can_go_back(controller.onboarding_can_go_back());
    ui.set_onboarding_orb_state(controller.onboarding_orb_state().stable_id().into());
    ui.set_selected_persona(controller.persona().stable_id().into());
    ui.set_orb_state(controller.orb_state().stable_id().into());
    ui.set_active_section(controller.active_section().stable_id().into());
    ui.set_section_title(section_title.into());
    ui.set_section_subtitle(section_subtitle.into());
    ui.set_routines_empty_text(controller.empty_routines_text().into());
    ui.set_awaiting_reply(controller.awaiting_reply());
    let (today_visible, conversation_visible, memory_visible, routines_visible) =
        controller.active_section().visibility();
    ui.set_today_visible(today_visible);
    ui.set_conversation_visible(conversation_visible);
    ui.set_memory_visible(memory_visible);
    ui.set_routines_visible(routines_visible);
}

/// Republishes the transcript the controller holds.
fn set_chat_rows(ui: &MainWindow, controller: &Controller) {
    let rows = controller
        .conversation()
        .iter()
        .map(|turn| ChatRow {
            role: turn.role.stable_id().into(),
            text: turn.text.clone().into(),
        })
        .collect::<Vec<_>>();
    ui.set_chat_rows(Rc::new(VecModel::from(rows)).into());
}

fn sync_orb_ui(ui: &OrbWindow, controller: &Controller) {
    ui.set_assistant_name(controller.assistant_name().into());
    ui.set_orb_state(controller.orb_state().stable_id().into());
    ui.set_orb_size(i32::from(controller.orb_size()));
}

fn sync_profile_ui(ui: &MainWindow, store: Option<&MemoryStore>) {
    let Ok(Some(profile)) = store.map_or(Ok(None), MemoryStore::load_profile) else {
        return;
    };

    ui.set_selected_profession(profile.professional_profile.stable_id().into());
    ui.set_selected_assistance(profile.assistance_style.stable_id().into());
}

fn sync_ai_ui(ui: &MainWindow, settings: &AiSettings, has_api_key: bool) {
    ui.set_ai_provider(settings.provider.stable_id().into());
    ui.set_endpoint_label(settings.provider.endpoint_label().into());
    ui.set_endpoint_placeholder(settings.provider.endpoint_placeholder().into());
    ui.set_local_endpoint(settings.endpoint.clone().into());
    ui.set_local_model_path(settings.model_path.clone().into());
    ui.set_has_api_key(has_api_key);
}

/// Points the settings panel at a provider and re-reads whether that provider has a key.
///
/// Each provider keeps its own credential, so the "saved" badge has to follow the selection
/// rather than describe whichever provider happened to be active at launch.
fn apply_provider_selection(ui: &MainWindow, context: &AppContext, provider: AiProvider) {
    ui.set_ai_provider(provider.stable_id().into());
    ui.set_endpoint_label(provider.endpoint_label().into());
    ui.set_endpoint_placeholder(provider.endpoint_placeholder().into());

    if let Ok(has_key) = provider_has_api_key(*context.vault, provider) {
        ui.set_has_api_key(has_key);
        ui.set_api_key_summary(api_key_summary(Some(provider), has_key).into());
        if provider.accepts_api_key() {
            ui.set_selected_key_provider(provider.stable_id().into());
            ui.set_key_editor_label(key_editor_label(provider.stable_id()));
        }
    } else {
        ui.set_has_api_key(false);
        ui.set_api_key_summary(String::new().into());
        let mut controller = context.controller.borrow_mut();
        controller.record_credential_error();
        sync_ui(ui, &controller);
    }
}

fn provider_has_api_key(
    vault: WindowsCredentialVault,
    provider: AiProvider,
) -> Result<bool, CredentialError> {
    if provider == AiProvider::Disabled {
        Ok(false)
    } else {
        vault.has_key(provider.stable_id())
    }
}

fn set_memory_rows(ui: &MainWindow, notes: Vec<Note>) {
    let rows = notes
        .into_iter()
        .map(|note| MemoryRow {
            title: note.title.into(),
            preview: note.body.into(),
            meta: "memoria local".into(),
        })
        .collect::<Vec<_>>();
    ui.set_memory_rows(Rc::new(VecModel::from(rows)).into());
}

fn initial_routines() -> Vec<RoutineRow> {
    vec![RoutineRow {
        title: "Preparar tu regreso al proyecto".into(),
        explanation: "Sueles abrir el editor y las notas del proyecto juntos al comenzar.".into(),
        action: "Abrir workspace + mostrar ultima memoria".into(),
    }]
}

fn connect_onboarding_callbacks(ui: &MainWindow, context: &AppContext) {
    let ui_weak = ui.as_weak();
    let persona_context = context.clone();
    ui.on_persona_selected(move |persona_id| {
        let mut controller = persona_context.controller.borrow_mut();
        match Persona::from_stable_id(persona_id.as_str()) {
            Some(persona) => controller.preview_persona(persona),
            None => controller.record_ai_validation_error(UNKNOWN_PERSONA),
        }
        sync_weak_ui(&ui_weak, &controller);
    });

    let ui_weak = ui.as_weak();
    let back_context = context.clone();
    ui.on_onboarding_back(move || {
        let mut controller = back_context.controller.borrow_mut();
        controller.back_onboarding();
        sync_weak_ui(&ui_weak, &controller);
    });

    let ui_weak = ui.as_weak();
    let replay_context = context.clone();
    ui.on_replay_onboarding(move || {
        let mut controller = replay_context.controller.borrow_mut();
        controller.replay_onboarding();
        sync_weak_ui(&ui_weak, &controller);
    });

    let ui_weak = ui.as_weak();
    let advance_context = context.clone();
    ui.on_onboarding_advance(move || handle_onboarding_advance(&advance_context, &ui_weak));
}

fn handle_onboarding_advance(context: &AppContext, ui_weak: &slint::Weak<MainWindow>) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };

    {
        let mut controller = context.controller.borrow_mut();
        if controller.advance_onboarding() == OnboardingAdvance::Moved {
            sync_ui(&ui, &controller);
            return;
        }
    }

    finish_onboarding(context, &ui);
}

fn finish_onboarding(context: &AppContext, ui: &MainWindow) {
    let answers = OnboardingAnswers::try_from_ui(
        ui.get_selected_persona().as_str(),
        ui.get_selected_profession().as_str(),
        ui.get_selected_assistance().as_str(),
        ui.get_selected_ai_setup().as_str(),
    );
    let answers = match answers {
        Ok(answers) => answers,
        Err(message) => {
            // The flow stays on its last question so the answer can be corrected in place.
            let mut controller = context.controller.borrow_mut();
            controller.record_ai_validation_error(message);
            sync_ui(ui, &controller);
            return;
        }
    };

    {
        let mut controller = context.controller.borrow_mut();
        controller.select_persona(answers.persona);
        if context
            .store()
            .is_none_or(|store| store.save_profile(&answers.profile()).is_err())
        {
            controller.record_storage_error();
        }
        sync_ui(ui, &controller);
    }

    let provider = answers.ai_setup.preselected_provider();
    if provider != AiProvider::Disabled {
        apply_provider_selection(ui, context, provider);
        ui.set_settings_visible(true);
    }
    ui.window().request_redraw();
}

#[derive(Clone, Copy)]
struct SavedWindowGeometry {
    position: PhysicalPosition,
    size: PhysicalSize,
}

fn connect_navigation_callback(ui: &MainWindow, controller: Rc<RefCell<Controller>>) {
    let ui_weak = ui.as_weak();
    ui.on_navigate_to(move |section| {
        let Some(section) = WorkspaceSection::parse(section.as_str()) else {
            return;
        };
        let mut controller = controller.borrow_mut();
        controller.navigate_to(section);
        sync_weak_ui(&ui_weak, &controller);
    });
}

fn connect_window_callbacks(
    ui: &MainWindow,
    orb_ui: &OrbWindow,
    controller: Rc<RefCell<Controller>>,
) {
    let saved_geometry = Rc::new(RefCell::new(None::<SavedWindowGeometry>));

    let ui_weak = ui.as_weak();
    let orb_weak = orb_ui.as_weak();
    let enter_controller = Rc::clone(&controller);
    let enter_geometry = Rc::clone(&saved_geometry);
    ui.on_enter_orb_mode(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        let Some(orb_ui) = orb_weak.upgrade() else {
            return;
        };
        let window = ui.window();
        let previous = SavedWindowGeometry {
            position: window.position(),
            size: window.size(),
        };
        *enter_geometry.borrow_mut() = Some(previous);

        let mut controller = enter_controller.borrow_mut();
        controller.enter_orb_mode();
        debug_assert_eq!(controller.window_mode(), WindowMode::Orb);
        sync_ui(&ui, &controller);
        sync_orb_ui(&orb_ui, &controller);

        let orb_size = controller.orb_size();
        let scale_factor = window.scale_factor();
        let logical_size = LogicalSize::new(f32::from(orb_size), f32::from(orb_size));
        let orb_width =
            i32::try_from(logical_size.to_physical(scale_factor).width).unwrap_or(i32::MAX);
        let previous_width = i32::try_from(previous.size.width).unwrap_or(i32::MAX);
        let (x, y) = anchored_orb_position(
            previous.position.x,
            previous.position.y,
            previous_width,
            orb_width,
        );
        orb_ui.window().set_size(logical_size);
        orb_ui.window().set_position(PhysicalPosition::new(x, y));
        let _ = orb_ui.show();
        let _ = ui.hide();
    });

    connect_orb_drag_callback(orb_ui);
    connect_orb_resize_callback(orb_ui, Rc::clone(&controller));

    let ui_weak = ui.as_weak();
    let orb_weak = orb_ui.as_weak();
    orb_ui.on_restore_dashboard(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        let Some(orb_ui) = orb_weak.upgrade() else {
            return;
        };
        let mut controller = controller.borrow_mut();
        controller.restore_dashboard();
        debug_assert_eq!(controller.window_mode(), WindowMode::Dashboard);
        sync_ui(&ui, &controller);

        if let Some(saved) = saved_geometry.borrow_mut().take() {
            ui.window().set_size(saved.size);
            ui.window().set_position(saved.position);
        } else {
            let (width, height) = WindowMode::Dashboard.logical_size();
            ui.window()
                .set_size(LogicalSize::new(f32::from(width), f32::from(height)));
        }
        let _ = ui.show();
        ui.window().request_redraw();
        let _ = orb_ui.hide();
    });
}

fn connect_orb_drag_callback(orb_ui: &OrbWindow) {
    let orb_weak = orb_ui.as_weak();
    orb_ui.on_drag_orb(move |delta_x, delta_y| {
        let Some(orb_ui) = orb_weak.upgrade() else {
            return;
        };
        let window = orb_ui.window();
        let delta = LogicalPosition::new(delta_x, delta_y).to_physical(window.scale_factor());
        let current = window.position();
        let (x, y) = translated_orb_position(current.x, current.y, delta.x, delta.y);
        window.set_position(PhysicalPosition::new(x, y));
    });
}

fn connect_orb_resize_callback(orb_ui: &OrbWindow, controller: Rc<RefCell<Controller>>) {
    let orb_weak = orb_ui.as_weak();
    orb_ui.on_resize_orb(move |action| {
        let Some(action) = OrbResizeAction::parse(action.as_str()) else {
            return;
        };
        let Some(orb_ui) = orb_weak.upgrade() else {
            return;
        };
        let window = orb_ui.window();
        let current_position = window.position();
        let current_size = i32::try_from(window.size().width).unwrap_or(i32::MAX);

        let mut controller = controller.borrow_mut();
        controller.resize_orb(action);
        sync_orb_ui(&orb_ui, &controller);

        let logical_size = LogicalSize::new(
            f32::from(controller.orb_size()),
            f32::from(controller.orb_size()),
        );
        let new_size = i32::try_from(logical_size.to_physical(window.scale_factor()).width)
            .unwrap_or(i32::MAX);
        let (x, y) = centered_resize_position(
            current_position.x,
            current_position.y,
            current_size,
            new_size,
        );
        window.set_size(logical_size);
        window.set_position(PhysicalPosition::new(x, y));
    });
}

fn connect_note_callback(ui: &MainWindow, context: &AppContext) {
    let ui_weak = ui.as_weak();
    let context = context.clone();
    ui.on_save_note(move |title, body| {
        let submission = NoteSubmission::try_from_ui(title.as_str(), body.as_str());
        let mut controller = context.controller.borrow_mut();

        match submission {
            Ok(submission) => {
                let note = Note::new(submission.title, submission.body);
                if context
                    .store()
                    .is_some_and(|store| store.save_note(&note).is_ok())
                {
                    controller.record_note_saved();
                } else {
                    controller.record_storage_error();
                }
            }
            Err(_) => controller.record_invalid_note(),
        }

        let notes = load_notes(context.store());
        controller.set_note_count(notes.len());
        if let Some(ui) = ui_weak.upgrade() {
            sync_ui(&ui, &controller);
            set_memory_rows(&ui, notes);
        }
    });
}

fn connect_routine_callbacks(
    ui: &MainWindow,
    controller: Rc<RefCell<Controller>>,
    routines: Rc<VecModel<RoutineRow>>,
) {
    let ui_weak = ui.as_weak();
    let accept_controller = Rc::clone(&controller);
    let accept_routines = Rc::clone(&routines);
    ui.on_accept_routine(move |index| {
        let mut controller = accept_controller.borrow_mut();
        controller.record_routine_accepted();
        remove_model_row(&accept_routines, index);
        sync_weak_ui(&ui_weak, &controller);
    });

    let ui_weak = ui.as_weak();
    ui.on_dismiss_routine(move |index| {
        let mut controller = controller.borrow_mut();
        controller.record_routine_dismissed();
        remove_model_row(&routines, index);
        sync_weak_ui(&ui_weak, &controller);
    });
}

fn remove_model_row<T: Clone + 'static>(model: &VecModel<T>, index: i32) {
    if let Ok(index) = usize::try_from(index)
        && index < model.row_count()
    {
        model.remove(index);
    }
}

fn connect_ai_callbacks(ui: &MainWindow, context: &AppContext) {
    let ui_weak = ui.as_weak();
    let select_context = context.clone();
    ui.on_provider_selected(move |provider_id| {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        let Some(provider) = AiProvider::from_stable_id(provider_id.as_str()) else {
            let mut controller = select_context.controller.borrow_mut();
            controller.record_ai_validation_error(UNKNOWN_PROVIDER);
            sync_ui(&ui, &controller);
            return;
        };
        apply_provider_selection(&ui, &select_context, provider);
    });

    let ui_weak = ui.as_weak();
    let save_context = context.clone();
    ui.on_save_ai_settings(move |provider, endpoint, model_path| {
        handle_save_ai_settings(&save_context, &ui_weak, &provider, &endpoint, &model_path);
    });

    let ui_weak = ui.as_weak();
    let key_select_context = context.clone();
    ui.on_key_provider_selected(move |provider_id| {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        if AiProvider::from_stable_id(provider_id.as_str()).is_none() {
            let mut controller = key_select_context.controller.borrow_mut();
            controller.record_ai_validation_error(UNKNOWN_PROVIDER);
            sync_ui(&ui, &controller);
            return;
        }
        // Selecting a row only aims the editor; no credential is read or written.
        ui.set_key_editor_label(key_editor_label(provider_id.as_str()));
        ui.set_selected_key_provider(provider_id);
    });

    let ui_weak = ui.as_weak();
    let store_context = context.clone();
    ui.on_store_api_key(move |provider, api_key| {
        handle_store_api_key(&store_context, &ui_weak, &provider, &api_key);
    });

    let ui_weak = ui.as_weak();
    let forget_context = context.clone();
    ui.on_forget_api_key(move |provider| {
        handle_forget_api_key(&forget_context, &ui_weak, &provider);
    });
}

/// Republishes the key manager from the vault, which is the only source of truth for it.
///
/// Called after every add and every delete so the list, the badge on the connection tab and
/// the stored state can never disagree with what Windows actually holds.
fn refresh_key_manager(ui: &MainWindow, context: &AppContext) {
    let Ok(inventory) = read_key_inventory(context.vault.as_ref()) else {
        let mut controller = context.controller.borrow_mut();
        controller.record_credential_error();
        sync_ui(ui, &controller);
        return;
    };

    ui.set_api_key_rows(api_key_rows(&inventory));
    ui.set_key_editor_label(key_editor_label(ui.get_selected_key_provider().as_str()));

    let selected = AiProvider::from_stable_id(ui.get_ai_provider().as_str());
    let has_key = selected.is_some_and(|provider| {
        inventory
            .iter()
            .any(|status| status.provider == provider && status.stored)
    });
    ui.set_has_api_key(has_key);
    ui.set_api_key_summary(api_key_summary(selected, has_key).into());

    let stored_keys = inventory.iter().filter(|status| status.stored).count();
    let settings = stored_ai_settings(context);
    let ready = settings.intelligence_is_available(stored_keys);
    ui.set_intelligence_ready(ready);
    ui.set_intelligence_hint(NO_INTELLIGENCE.into());
    if !ready {
        // Every path that can change this ends here, so the warning cannot go stale.
        let mut controller = context.controller.borrow_mut();
        controller.record_missing_intelligence();
        sync_ui(ui, &controller);
    }
}

/// The connection as it is actually persisted, not as the form currently reads.
fn stored_ai_settings(context: &AppContext) -> AiSettings {
    context
        .store()
        .and_then(|store| store.load_ai_settings().ok())
        .unwrap_or_default()
}

/// Says what the connection tab knows about the selected provider's credential.
fn api_key_summary(provider: Option<AiProvider>, has_key: bool) -> String {
    let Some(provider) = provider else {
        return String::new();
    };
    match provider {
        AiProvider::Disabled => {
            "Sin proveedor activo. Tom sigue funcionando con tu memoria local.".to_owned()
        }
        provider if has_key => format!("{} tiene una clave guardada en Windows.", provider.label()),
        provider => format!(
            "{} no tiene clave guardada. Puedes anadir una o usar un modelo local.",
            provider.label()
        ),
    }
}

fn key_editor_label(provider_id: &str) -> slint::SharedString {
    AiProvider::from_stable_id(provider_id).map_or_else(
        || "Elige un proveedor de la lista".into(),
        |provider| format!("Añadir o reemplazar la clave de {}", provider.label()).into(),
    )
}

fn handle_store_api_key(
    context: &AppContext,
    ui_weak: &slint::Weak<MainWindow>,
    provider: &slint::SharedString,
    api_key: &slint::SharedString,
) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };
    let submission = match ApiKeySubmission::try_from_ui(provider.as_str(), api_key.as_str()) {
        Ok(submission) => submission,
        Err(message) => {
            let mut controller = context.controller.borrow_mut();
            controller.record_ai_validation_error(message);
            sync_ui(&ui, &controller);
            return;
        }
    };

    {
        let mut controller = context.controller.borrow_mut();
        if store_api_key(
            context.vault.as_ref(),
            submission.provider.stable_id(),
            &submission.api_key,
        )
        .is_err()
        {
            controller.record_credential_error();
        } else {
            controller.record_api_key_stored();
        }
        sync_ui(&ui, &controller);
    }

    refresh_key_manager(&ui, context);
}

fn handle_forget_api_key(
    context: &AppContext,
    ui_weak: &slint::Weak<MainWindow>,
    provider: &slint::SharedString,
) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };
    {
        let mut controller = context.controller.borrow_mut();
        if AiProvider::from_stable_id(provider.as_str()).is_none() {
            controller.record_ai_validation_error(UNKNOWN_PROVIDER);
            sync_ui(&ui, &controller);
            return;
        }
        // Only this provider's key goes; every other credential stays where it is.
        if forget_api_key(context.vault.as_ref(), provider.as_str()).is_err() {
            controller.record_credential_error();
        } else {
            controller.record_api_key_cleared();
        }
        sync_ui(&ui, &controller);
    }

    refresh_key_manager(&ui, context);
}

fn handle_save_ai_settings(
    context: &AppContext,
    ui_weak: &slint::Weak<MainWindow>,
    provider: &slint::SharedString,
    endpoint: &slint::SharedString,
    model_path: &slint::SharedString,
) {
    let Some(provider_choice) = AiProvider::from_stable_id(provider.as_str()) else {
        let mut controller = context.controller.borrow_mut();
        controller.record_ai_validation_error(UNKNOWN_PROVIDER);
        sync_weak_ui(ui_weak, &controller);
        return;
    };
    let Ok(has_stored_key) = provider_has_api_key(*context.vault, provider_choice) else {
        let mut controller = context.controller.borrow_mut();
        controller.record_credential_error();
        sync_weak_ui(ui_weak, &controller);
        return;
    };
    let submission = match AiSettingsSubmission::try_from_ui(
        provider.as_str(),
        endpoint.as_str(),
        model_path.as_str(),
    ) {
        Ok(submission) => submission,
        Err(message) => {
            let mut controller = context.controller.borrow_mut();
            controller.record_ai_validation_error(message);
            if let Some(ui) = ui_weak.upgrade() {
                // A refused save leaves the panel open so it can be corrected in place.
                ui.set_settings_visible(true);
                sync_ui(&ui, &controller);
            }
            return;
        }
    };
    let Some(store) = context.store() else {
        let mut controller = context.controller.borrow_mut();
        controller.record_storage_error();
        sync_weak_ui(ui_weak, &controller);
        return;
    };
    let settings = AiSettings {
        provider: submission.provider,
        endpoint: submission.endpoint,
        model_path: submission.model_path,
    };
    {
        let mut controller = context.controller.borrow_mut();
        if store.save_ai_settings(&settings).is_err() {
            controller.record_storage_error();
        } else {
            controller.record_ai_settings_saved();
        }
        if let Some(ui) = ui_weak.upgrade() {
            sync_ai_ui(&ui, &settings, has_stored_key);
            sync_ui(&ui, &controller);
        }
    }

    if let Some(ui) = ui_weak.upgrade() {
        refresh_key_manager(&ui, context);
    }
}

fn connect_conversation_callbacks(ui: &MainWindow, context: &AppContext) {
    let ui_weak = ui.as_weak();
    let send_context = context.clone();
    ui.on_send_message(move |question| {
        handle_send_message(&send_context, &ui_weak, question.as_str());
    });

    let ui_weak = ui.as_weak();
    let clear_context = context.clone();
    ui.on_clear_conversation(move || {
        let mut controller = clear_context.controller.borrow_mut();
        controller.clear_conversation();
        if let Some(ui) = ui_weak.upgrade() {
            set_chat_rows(&ui, &controller);
            sync_ui(&ui, &controller);
        }
    });

    // The worker thread cannot touch the controller, so it hands its result back through
    // this callback, which runs on the UI thread where the controller lives.
    let ui_weak = ui.as_weak();
    let reply_context = context.clone();
    ui.on_reply_arrived(move |succeeded, text| {
        let mut controller = reply_context.controller.borrow_mut();
        if succeeded {
            controller.record_reply(text.to_string());
        } else {
            controller.record_inference_error(text.to_string());
        }
        if let Some(ui) = ui_weak.upgrade() {
            set_chat_rows(&ui, &controller);
            sync_ui(&ui, &controller);
        }
    });
}

fn handle_send_message(context: &AppContext, ui_weak: &slint::Weak<MainWindow>, question: &str) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };
    // One exchange at a time: a second request would race the first into the transcript.
    if context.controller.borrow().awaiting_reply() {
        return;
    }

    let question = match ChatSubmission::try_from_ui(question) {
        Ok(submission) => submission.question,
        Err(message) => {
            let mut controller = context.controller.borrow_mut();
            controller.record_ai_validation_error(message);
            sync_ui(&ui, &controller);
            return;
        }
    };

    let settings = stored_ai_settings(context);
    let profile = stored_profile(context);

    // The credential is read here, on the UI thread, and lives only inside the request
    // that is about to be sent.
    let Ok(api_key) = read_provider_key(context, settings.provider) else {
        let mut controller = context.controller.borrow_mut();
        controller.record_credential_error();
        sync_ui(&ui, &controller);
        return;
    };

    let mut controller = context.controller.borrow_mut();
    controller.begin_exchange(question);
    let request = match build_chat_request(
        &settings,
        api_key.as_deref(),
        &system_prompt(&profile),
        controller.conversation(),
    ) {
        Ok(request) => request,
        Err(error) => {
            controller.record_inference_error(error.user_message());
            set_chat_rows(&ui, &controller);
            sync_ui(&ui, &controller);
            return;
        }
    };
    set_chat_rows(&ui, &controller);
    sync_ui(&ui, &controller);
    drop(controller);

    let ui_weak = ui_weak.clone();
    std::thread::spawn(move || {
        let outcome = transport::send_chat(&request);
        // If the window is gone there is nobody left to tell.
        let _ = ui_weak.upgrade_in_event_loop(move |ui| match outcome {
            Ok(reply) => ui.invoke_reply_arrived(true, reply.into()),
            Err(error) => ui.invoke_reply_arrived(false, error.user_message().into()),
        });
    });
}

/// Reads the credential for a provider, or `Err` when the vault itself failed.
///
/// A provider with no stored key is `Ok(None)`: local models are reached without one, and
/// a cloud provider's missing key is reported by the request builder with the precise
/// reason rather than as a vault failure.
fn read_provider_key(context: &AppContext, provider: AiProvider) -> Result<Option<String>, ()> {
    if !provider.accepts_api_key() {
        return Ok(None);
    }
    context.vault.read_key(provider.stable_id()).map_err(|_| ())
}

/// The profile the system prompt is built from, falling back to defaults before onboarding.
fn stored_profile(context: &AppContext) -> UserProfile {
    context
        .store()
        .and_then(|store| store.load_profile().ok().flatten())
        .unwrap_or_default()
}

fn sync_weak_ui(ui_weak: &slint::Weak<MainWindow>, controller: &Controller) {
    if let Some(ui) = ui_weak.upgrade() {
        sync_ui(&ui, controller);
    }
}
