use tom_core::{AiProvider, AiSettings, MemoryStore};

#[test]
fn ai_provider_ids_are_stable_and_round_trip() {
    let cases = [
        (AiProvider::Disabled, "disabled"),
        (AiProvider::OpenAi, "openai"),
        (AiProvider::Anthropic, "anthropic"),
        (AiProvider::Gemini, "gemini"),
        (AiProvider::CustomCloud, "custom-cloud"),
        (AiProvider::Ollama, "ollama"),
        (AiProvider::LmStudio, "lm-studio"),
        (AiProvider::CustomLocal, "custom-local"),
    ];

    for (provider, stable_id) in cases {
        assert_eq!(provider.stable_id(), stable_id);
        assert_eq!(AiProvider::from_stable_id(stable_id), Some(provider));
    }
    assert_eq!(AiProvider::from_stable_id("unknown"), None);
}

#[test]
fn ai_settings_default_to_disabled_without_locations() {
    assert_eq!(
        AiSettings::default(),
        AiSettings {
            provider: AiProvider::Disabled,
            endpoint: String::new(),
            model_path: String::new(),
        }
    );
}

#[test]
fn a_new_store_returns_disabled_ai_settings() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");

    assert_eq!(
        store.load_ai_settings().expect("settings should load"),
        AiSettings::default()
    );
}

#[test]
fn ai_settings_round_trip_without_an_api_key() {
    let store = MemoryStore::open_in_memory().expect("store should initialize");
    let settings = AiSettings {
        provider: AiProvider::Ollama,
        endpoint: "http://127.0.0.1:11434".to_owned(),
        model_path: "qwen3:8b".to_owned(),
    };

    store
        .save_ai_settings(&settings)
        .expect("settings should save");

    assert_eq!(
        store.load_ai_settings().expect("settings should load"),
        settings
    );
}

#[test]
fn ai_settings_schema_contains_no_secret_column() {
    let database_path = std::env::temp_dir().join(format!(
        "tom-core-ai-settings-{}.sqlite3",
        uuid::Uuid::now_v7()
    ));
    let store = MemoryStore::open(&database_path).expect("store should initialize");
    drop(store);

    let connection = rusqlite::Connection::open(&database_path).expect("database should open");
    let mut statement = connection
        .prepare("PRAGMA table_info(ai_settings)")
        .expect("schema should be readable");
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("columns should be queryable")
        .collect::<Result<Vec<_>, _>>()
        .expect("columns should load");
    drop(statement);
    drop(connection);
    std::fs::remove_file(database_path).expect("temporary database should be removable");

    assert_eq!(columns, ["singleton", "provider", "endpoint", "model_path"]);
}
