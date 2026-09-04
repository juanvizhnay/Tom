use tom_core::{AssistanceStyle, MemoryStore, Note, Persona, ProfessionalProfile, UserProfile};

#[test]
fn a_new_store_has_no_profile_or_notes() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");

    assert_eq!(
        store.load_profile().expect("profile query should work"),
        None
    );
    assert!(
        store
            .list_notes()
            .expect("notes query should work")
            .is_empty()
    );
}

#[test]
fn profile_round_trips_through_sqlite() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");
    let profile = UserProfile {
        persona: Persona::Tomy,
        professional_profile: ProfessionalProfile::Creator,
        assistance_style: AssistanceStyle::Proactive,
    };

    store.save_profile(&profile).expect("profile should save");

    assert_eq!(
        store.load_profile().expect("profile should load"),
        Some(profile)
    );
}

#[test]
fn a_legacy_profile_without_assistance_style_loads_as_balanced() {
    let path = std::env::temp_dir().join(format!("tom-legacy-{}.sqlite3", uuid::Uuid::now_v7()));
    {
        let connection = rusqlite::Connection::open(&path).expect("legacy database should open");
        connection
            .execute_batch(
                "CREATE TABLE user_profile (
                    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                    persona TEXT NOT NULL,
                    professional_profile TEXT NOT NULL
                );
                INSERT INTO user_profile (singleton, persona, professional_profile)
                VALUES (1, 'tom', 'developer');",
            )
            .expect("legacy profile should be created");
    }

    let profile = MemoryStore::open(&path)
        .expect("store should migrate")
        .load_profile()
        .expect("profile should load")
        .expect("profile should exist");

    assert_eq!(profile.assistance_style, AssistanceStyle::Balanced);
    drop(std::fs::remove_file(path));
}

#[test]
fn notes_are_saved_and_listed_newest_first() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");
    let older = Note::new("Morning", "Check the issue tracker");
    let mut newer = Note::new("Afternoon", "Review pull requests");
    newer.created_at = older.created_at + chrono::Duration::seconds(1);
    newer.updated_at = newer.created_at;

    store.save_note(&older).expect("older note should save");
    store.save_note(&newer).expect("newer note should save");

    let notes = store.list_notes().expect("notes should load");
    assert_eq!(notes, vec![newer, older]);
}

#[test]
fn saving_an_existing_note_updates_it_without_duplication() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");
    let mut note = Note::new("Daily plan", "Open the board");
    store.save_note(&note).expect("note should save");

    note.body = "Open the board and review blockers".to_owned();
    note.updated_at += chrono::Duration::seconds(1);
    store.save_note(&note).expect("note should update");

    assert_eq!(store.list_notes().expect("notes should load"), vec![note]);
}

#[test]
fn search_finds_case_insensitive_matches_in_title_or_body() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");
    let rust_note = Note::new("Rust release", "Read the migration guide");
    let meeting_note = Note::new("Team meeting", "Discuss API design");
    store.save_note(&rust_note).expect("note should save");
    store.save_note(&meeting_note).expect("note should save");

    assert_eq!(
        store.search_notes("rust").expect("search should work"),
        vec![rust_note]
    );
    assert_eq!(
        store.search_notes("api").expect("search should work"),
        vec![meeting_note]
    );
}

#[test]
fn an_empty_search_returns_all_notes() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");
    let note = Note::new("Ideas", "Capture project ideas");
    store.save_note(&note).expect("note should save");

    assert_eq!(
        store.search_notes("   ").expect("search should work"),
        vec![note]
    );
}
