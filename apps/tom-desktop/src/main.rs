#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod controller;
mod credential;

use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

use controller::{
    AiProviderChoice, AiSettingsSubmission, Controller, NoteSubmission, OrbResizeAction, OrbState,
    PersonaChoice, WindowMode, WorkspaceSection, anchored_orb_position, centered_resize_position,
    translated_orb_position,
};
use credential::{
    CredentialError, CredentialVault, WindowsCredentialVault, clear_api_key, persist_api_key,
};
use directories::ProjectDirs;
use slint::{
    ComponentHandle, LogicalPosition, LogicalSize, Model, PhysicalPosition, PhysicalSize, VecModel,
};
use tom_core::{
    AiProvider, AiSettings, AssistanceStyle, MemoryStore, Note, Persona, ProfessionalProfile,
    UserProfile,
};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let orb_ui = OrbWindow::new()?;
    let store = Rc::new(open_memory_store());
    let notes = load_notes(store.as_ref().as_ref());
    let mut initial_controller = load_controller(store.as_ref().as_ref(), notes.len());
    let ai_settings = load_ai_settings(store.as_ref().as_ref(), &mut initial_controller);
    let credential_vault = Rc::new(WindowsCredentialVault);
    let has_api_key =
        provider_has_api_key(*credential_vault, ai_settings.provider).unwrap_or_else(|_| {
            initial_controller.record_credential_error();
            false
        });
    let controller = Rc::new(RefCell::new(initial_controller));
    let routine_model = Rc::new(VecModel::from(initial_routines()));

    sync_ui(&ui, &controller.borrow());
    sync_orb_ui(&orb_ui, &controller.borrow());
    sync_profile_ui(&ui, store.as_ref().as_ref());
    sync_ai_ui(&ui, &ai_settings, has_api_key);
    set_memory_rows(&ui, notes);
    ui.set_routine_rows(Rc::clone(&routine_model).into());
    connect_onboarding_callback(&ui, Rc::clone(&controller), Rc::clone(&store));
    connect_navigation_callback(&ui, Rc::clone(&controller));
    connect_window_callbacks(&ui, &orb_ui, Rc::clone(&controller));
    connect_note_callback(&ui, Rc::clone(&controller), Rc::clone(&store));
    connect_routine_callbacks(&ui, Rc::clone(&controller), routine_model);
    connect_ai_callbacks(&ui, Rc::clone(&controller), store, credential_vault);

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
        Ok(Some(profile)) => Controller::restored(persona_choice(profile.persona), note_count),
        Ok(None) => Controller::default(),
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

const fn persona_choice(persona: Persona) -> PersonaChoice {
    match persona {
        Persona::Tom => PersonaChoice::Tom,
        Persona::Tomy => PersonaChoice::Tomy,
    }
}

const fn core_persona(persona: PersonaChoice) -> Persona {
    match persona {
        PersonaChoice::Tom => Persona::Tom,
        PersonaChoice::Tomy => Persona::Tomy,
    }
}

const fn core_ai_provider(provider: AiProviderChoice) -> AiProvider {
    match provider {
        AiProviderChoice::Disabled => AiProvider::Disabled,
        AiProviderChoice::OpenAi => AiProvider::OpenAi,
        AiProviderChoice::Anthropic => AiProvider::Anthropic,
        AiProviderChoice::Gemini => AiProvider::Gemini,
        AiProviderChoice::CustomCloud => AiProvider::CustomCloud,
        AiProviderChoice::Ollama => AiProvider::Ollama,
        AiProviderChoice::LmStudio => AiProvider::LmStudio,
        AiProviderChoice::CustomLocal => AiProvider::CustomLocal,
    }
}

fn sync_ui(ui: &MainWindow, controller: &Controller) {
    debug_assert!(OrbState::ALL.contains(&controller.orb_state()));
    debug_assert!(WorkspaceSection::ALL.contains(&controller.active_section()));
    ui.set_assistant_name(controller.assistant_name().into());
    ui.set_status_text(controller.status_text().into());
    ui.set_note_count(controller.note_count());
    ui.set_onboarding_visible(controller.onboarding_visible());
    ui.set_orb_state(controller.orb_state().stable_id().into());
    ui.set_active_section(controller.active_section().stable_id().into());
    let (today_visible, memory_visible, routines_visible) =
        controller.active_section().visibility();
    ui.set_today_visible(today_visible);
    ui.set_memory_visible(memory_visible);
    ui.set_routines_visible(routines_visible);
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
    ui.set_local_endpoint(settings.endpoint.clone().into());
    ui.set_local_model_path(settings.model_path.clone().into());
    ui.set_has_api_key(has_api_key);
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

fn connect_onboarding_callback(
    ui: &MainWindow,
    controller: Rc<RefCell<Controller>>,
    store: Rc<Option<MemoryStore>>,
) {
    let ui_weak = ui.as_weak();
    ui.on_onboarding_completed(move |persona, profession, assistance| {
        let persona = PersonaChoice::parse(persona.as_str());
        let Some(professional_profile) = ProfessionalProfile::from_stable_id(profession.as_str())
        else {
            let mut controller = controller.borrow_mut();
            controller.record_ai_validation_error("Perfil profesional no reconocido");
            sync_weak_ui(&ui_weak, &controller);
            return;
        };
        let Some(assistance_style) = AssistanceStyle::from_stable_id(assistance.as_str()) else {
            let mut controller = controller.borrow_mut();
            controller.record_ai_validation_error("Nivel de iniciativa no reconocido");
            sync_weak_ui(&ui_weak, &controller);
            return;
        };
        let profile = UserProfile {
            persona: core_persona(persona),
            professional_profile,
            assistance_style,
        };

        let mut controller = controller.borrow_mut();
        controller.select_persona(persona);
        if store
            .as_ref()
            .as_ref()
            .is_none_or(|store| store.save_profile(&profile).is_err())
        {
            controller.record_storage_error();
        }

        if let Some(ui) = ui_weak.upgrade() {
            sync_ui(&ui, &controller);
            ui.window().request_redraw();
        }
    });
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
        if let Some(ui) = ui_weak.upgrade() {
            sync_ui(&ui, &controller);
        }
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

fn connect_note_callback(
    ui: &MainWindow,
    controller: Rc<RefCell<Controller>>,
    store: Rc<Option<MemoryStore>>,
) {
    let ui_weak = ui.as_weak();
    ui.on_save_note(move |title, body| {
        let submission = NoteSubmission::try_from_ui(title.as_str(), body.as_str());
        let mut controller = controller.borrow_mut();

        match submission {
            Ok(submission) => {
                let note = Note::new(submission.title, submission.body);
                if store
                    .as_ref()
                    .as_ref()
                    .is_some_and(|store| store.save_note(&note).is_ok())
                {
                    controller.record_note_saved();
                } else {
                    controller.record_storage_error();
                }
            }
            Err(_) => controller.record_invalid_note(),
        }

        if let Some(ui) = ui_weak.upgrade() {
            sync_ui(&ui, &controller);
            set_memory_rows(&ui, load_notes(store.as_ref().as_ref()));
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
        if let Some(ui) = ui_weak.upgrade() {
            sync_ui(&ui, &controller);
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_dismiss_routine(move |index| {
        let mut controller = controller.borrow_mut();
        controller.record_routine_dismissed();
        remove_model_row(&routines, index);
        if let Some(ui) = ui_weak.upgrade() {
            sync_ui(&ui, &controller);
        }
    });
}

fn remove_model_row<T: Clone + 'static>(model: &VecModel<T>, index: i32) {
    if let Ok(index) = usize::try_from(index)
        && index < model.row_count()
    {
        model.remove(index);
    }
}

#[derive(Clone)]
struct AiContext {
    controller: Rc<RefCell<Controller>>,
    store: Rc<Option<MemoryStore>>,
    credential_vault: Rc<WindowsCredentialVault>,
}

fn connect_ai_callbacks(
    ui: &MainWindow,
    controller: Rc<RefCell<Controller>>,
    store: Rc<Option<MemoryStore>>,
    credential_vault: Rc<WindowsCredentialVault>,
) {
    let context = AiContext {
        controller,
        store,
        credential_vault,
    };
    let ui_weak = ui.as_weak();
    let save_context = context.clone();
    ui.on_save_ai_settings(move |provider, api_key, endpoint, model_path| {
        handle_save_ai_settings(
            &save_context,
            &ui_weak,
            &provider,
            &api_key,
            &endpoint,
            &model_path,
        );
    });

    let ui_weak = ui.as_weak();
    ui.on_clear_api_key(move || handle_clear_api_key(&context, &ui_weak));
}

fn handle_save_ai_settings(
    context: &AiContext,
    ui_weak: &slint::Weak<MainWindow>,
    provider: &slint::SharedString,
    api_key: &slint::SharedString,
    endpoint: &slint::SharedString,
    model_path: &slint::SharedString,
) {
    let Some(provider_choice) = AiProviderChoice::parse(provider.as_str()) else {
        let mut controller = context.controller.borrow_mut();
        controller.record_ai_validation_error("Proveedor de IA no reconocido");
        sync_weak_ui(ui_weak, &controller);
        return;
    };
    let Ok(has_existing_key) =
        provider_has_api_key(*context.credential_vault, core_ai_provider(provider_choice))
    else {
        let mut controller = context.controller.borrow_mut();
        controller.record_credential_error();
        sync_weak_ui(ui_weak, &controller);
        return;
    };
    let submission = match AiSettingsSubmission::try_from_ui(
        provider.as_str(),
        api_key.as_str(),
        endpoint.as_str(),
        model_path.as_str(),
        has_existing_key,
    ) {
        Ok(submission) => submission,
        Err(message) => {
            let mut controller = context.controller.borrow_mut();
            controller.record_ai_validation_error(message);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_has_api_key(has_existing_key);
                sync_ui(&ui, &controller);
            }
            return;
        }
    };
    let Some(store) = context.store.as_ref().as_ref() else {
        let mut controller = context.controller.borrow_mut();
        controller.record_storage_error();
        sync_weak_ui(ui_weak, &controller);
        return;
    };
    let has_api_key = if submission.provider == AiProviderChoice::Disabled {
        false
    } else {
        let Ok(has_key) = persist_api_key(
            context.credential_vault.as_ref(),
            submission.provider.stable_id(),
            submission.new_api_key.as_deref(),
        ) else {
            let mut controller = context.controller.borrow_mut();
            controller.record_credential_error();
            sync_weak_ui(ui_weak, &controller);
            return;
        };
        has_key
    };
    let settings = AiSettings {
        provider: core_ai_provider(submission.provider),
        endpoint: submission.endpoint,
        model_path: submission.model_path,
    };
    let mut controller = context.controller.borrow_mut();
    if store.save_ai_settings(&settings).is_err() {
        controller.record_storage_error();
    } else {
        controller.record_ai_settings_saved();
    }
    if let Some(ui) = ui_weak.upgrade() {
        sync_ai_ui(&ui, &settings, has_api_key);
        sync_ui(&ui, &controller);
    }
}

fn handle_clear_api_key(context: &AiContext, ui_weak: &slint::Weak<MainWindow>) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };
    let provider_id = ui.get_ai_provider();
    let mut controller = context.controller.borrow_mut();
    if AiProviderChoice::parse(provider_id.as_str()).is_none() {
        controller.record_ai_validation_error("Proveedor de IA no reconocido");
    } else if clear_api_key(context.credential_vault.as_ref(), provider_id.as_str()).is_err() {
        controller.record_credential_error();
    } else {
        controller.record_api_key_cleared();
        ui.set_has_api_key(false);
    }
    sync_ui(&ui, &controller);
}

fn sync_weak_ui(ui_weak: &slint::Weak<MainWindow>, controller: &Controller) {
    if let Some(ui) = ui_weak.upgrade() {
        sync_ui(&ui, controller);
    }
}
