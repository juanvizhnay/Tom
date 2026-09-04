#![forbid(unsafe_code)]

mod model;
mod store;

pub use model::{
    AiProvider, AiSettings, AssistanceStyle, Note, Persona, ProfessionalProfile, RoutineSuggestion,
    UserProfile,
};
pub use store::{MemoryStore, StoreError};
