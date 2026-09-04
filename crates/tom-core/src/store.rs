use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    AiProvider, AiSettings, AssistanceStyle, Note, Persona, ProfessionalProfile, UserProfile,
};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("SQLite operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("stored note has an invalid UUID: {value}")]
    InvalidUuid {
        value: String,
        #[source]
        source: uuid::Error,
    },
    #[error("stored note has an invalid timestamp: {value}")]
    InvalidTimestamp {
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    #[error("stored profile has an unknown persona: {0}")]
    UnknownPersona(String),
    #[error("stored profile has an unknown professional profile: {0}")]
    UnknownProfessionalProfile(String),
    #[error("stored profile has an unknown assistance style: {0}")]
    UnknownAssistanceStyle(String),
    #[error("stored AI settings have an unknown provider: {0}")]
    UnknownAiProvider(String),
}

pub struct MemoryStore {
    connection: Connection,
}

impl MemoryStore {
    /// Opens or creates a SQLite-backed memory store at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] when `SQLite` cannot open or initialize the store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path)?)
    }

    /// Opens an isolated in-memory store.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] when `SQLite` cannot initialize the schema.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, StoreError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_profile (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                persona TEXT NOT NULL,
                professional_profile TEXT NOT NULL,
                assistance_style TEXT NOT NULL DEFAULT 'balanced'
            );
            CREATE TABLE IF NOT EXISTS ai_settings (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                provider TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                model_path TEXT NOT NULL
            );",
        )?;
        let has_assistance_style = {
            let mut statement = connection.prepare("PRAGMA table_info(user_profile)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            columns
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .any(|column| column == "assistance_style")
        };
        if !has_assistance_style {
            connection.execute(
                "ALTER TABLE user_profile ADD COLUMN assistance_style TEXT NOT NULL DEFAULT 'balanced'",
                [],
            )?;
        }
        Ok(Self { connection })
    }

    /// Inserts a note or replaces the stored values for its identifier.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] when `SQLite` cannot persist the note.
    pub fn save_note(&self, note: &Note) -> Result<(), StoreError> {
        self.connection.execute(
            "INSERT INTO notes (id, title, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                 title = excluded.title,
                 body = excluded.body,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                note.id.to_string(),
                note.title,
                note.body,
                note.created_at.to_rfc3339(),
                note.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Loads all notes, ordered from newest to oldest.
    ///
    /// # Errors
    ///
    /// Returns a [`StoreError`] when `SQLite` cannot read the notes or stored data is invalid.
    pub fn list_notes(&self) -> Result<Vec<Note>, StoreError> {
        self.query_notes(
            "SELECT id, title, body, created_at, updated_at
             FROM notes ORDER BY created_at DESC, id DESC",
            None,
        )
    }

    /// Finds notes whose title or body contains `query`, ignoring ASCII case.
    /// An empty query has the same behavior as [`Self::list_notes`].
    ///
    /// # Errors
    ///
    /// Returns a [`StoreError`] when `SQLite` cannot search the notes or stored data is invalid.
    pub fn search_notes(&self, query: &str) -> Result<Vec<Note>, StoreError> {
        let query = query.trim();
        if query.is_empty() {
            return self.list_notes();
        }

        let pattern = format!("%{}%", escape_like(query));
        self.query_notes(
            "SELECT id, title, body, created_at, updated_at
             FROM notes
             WHERE title LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                OR body LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY created_at DESC, id DESC",
            Some(&pattern),
        )
    }

    /// Persists the single active user profile.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] when `SQLite` cannot persist the profile.
    pub fn save_profile(&self, profile: &UserProfile) -> Result<(), StoreError> {
        self.connection.execute(
            "INSERT INTO user_profile (singleton, persona, professional_profile, assistance_style)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(singleton) DO UPDATE SET
                 persona = excluded.persona,
                 professional_profile = excluded.professional_profile,
                 assistance_style = excluded.assistance_style",
            params![
                persona_to_str(profile.persona),
                profile.professional_profile.stable_id(),
                profile.assistance_style.stable_id(),
            ],
        )?;
        Ok(())
    }

    /// Loads the active user profile, if one has been configured.
    ///
    /// # Errors
    ///
    /// Returns a [`StoreError`] when `SQLite` cannot read the profile or stored data is invalid.
    pub fn load_profile(&self) -> Result<Option<UserProfile>, StoreError> {
        let stored = self
            .connection
            .query_row(
                "SELECT persona, professional_profile, assistance_style
                 FROM user_profile WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;

        stored
            .map(|(persona, professional_profile, assistance_style)| {
                Ok(UserProfile {
                    persona: parse_persona(&persona)?,
                    professional_profile: parse_professional_profile(&professional_profile)?,
                    assistance_style: AssistanceStyle::from_stable_id(&assistance_style)
                        .ok_or(StoreError::UnknownAssistanceStyle(assistance_style))?,
                })
            })
            .transpose()
    }

    /// Persists non-secret configuration for the active AI provider.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] when `SQLite` cannot persist the settings.
    pub fn save_ai_settings(&self, settings: &AiSettings) -> Result<(), StoreError> {
        self.connection.execute(
            "INSERT INTO ai_settings (singleton, provider, endpoint, model_path)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(singleton) DO UPDATE SET
                 provider = excluded.provider,
                 endpoint = excluded.endpoint,
                 model_path = excluded.model_path",
            params![
                settings.provider.stable_id(),
                settings.endpoint,
                settings.model_path,
            ],
        )?;
        Ok(())
    }

    /// Loads the active AI configuration or disabled defaults when none exists.
    ///
    /// # Errors
    ///
    /// Returns a [`StoreError`] when `SQLite` cannot read the settings or the stored provider is
    /// unknown.
    pub fn load_ai_settings(&self) -> Result<AiSettings, StoreError> {
        let stored = self
            .connection
            .query_row(
                "SELECT provider, endpoint, model_path
                 FROM ai_settings WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;

        stored.map_or_else(
            || Ok(AiSettings::default()),
            |(provider, endpoint, model_path)| {
                let provider = AiProvider::from_stable_id(&provider)
                    .ok_or(StoreError::UnknownAiProvider(provider))?;
                Ok(AiSettings {
                    provider,
                    endpoint,
                    model_path,
                })
            },
        )
    }

    fn query_notes(&self, sql: &str, pattern: Option<&str>) -> Result<Vec<Note>, StoreError> {
        let mut statement = self.connection.prepare(sql)?;
        let rows = match pattern {
            Some(pattern) => statement.query_map([pattern], read_stored_note)?,
            None => statement.query_map([], read_stored_note)?,
        };
        let stored = rows.collect::<Result<Vec<_>, _>>()?;
        stored.into_iter().map(decode_note).collect()
    }
}

type StoredNote = (String, String, String, String, String);

fn read_stored_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredNote> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn decode_note(stored: StoredNote) -> Result<Note, StoreError> {
    let (id, title, body, created_at, updated_at) = stored;
    Ok(Note {
        id: parse_uuid(id)?,
        title,
        body,
        created_at: parse_timestamp(created_at)?,
        updated_at: parse_timestamp(updated_at)?,
    })
}

fn parse_uuid(value: String) -> Result<Uuid, StoreError> {
    Uuid::parse_str(&value).map_err(|source| StoreError::InvalidUuid { value, source })
}

fn parse_timestamp(value: String) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(&value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|source| StoreError::InvalidTimestamp { value, source })
}

const fn persona_to_str(persona: Persona) -> &'static str {
    match persona {
        Persona::Tom => "tom",
        Persona::Tomy => "tomy",
    }
}

fn parse_persona(value: &str) -> Result<Persona, StoreError> {
    match value {
        "tom" => Ok(Persona::Tom),
        "tomy" => Ok(Persona::Tomy),
        _ => Err(StoreError::UnknownPersona(value.to_owned())),
    }
}

fn parse_professional_profile(value: &str) -> Result<ProfessionalProfile, StoreError> {
    ProfessionalProfile::from_stable_id(value)
        .ok_or_else(|| StoreError::UnknownProfessionalProfile(value.to_owned()))
}

fn escape_like(query: &str) -> String {
    query
        .chars()
        .flat_map(|character| match character {
            '%' => ['\\', '%'].into_iter().take(2),
            '_' => ['\\', '_'].into_iter().take(2),
            '\\' => ['\\', '\\'].into_iter().take(2),
            _ => [character, '\0'].into_iter().take(1),
        })
        .collect()
}
