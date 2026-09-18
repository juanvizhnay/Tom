#![forbid(unsafe_code)]

mod catalog;
mod model;
mod store;

pub use catalog::{
    AiSetup, Choice, OnboardingStep, PulseEntry, PulseTone, api_key_providers,
    cloud_provider_choices, local_provider_choices, pulse_timeline, welcome_highlights,
};
pub use model::{
    AiProvider, AiSettings, AssistanceStyle, Note, Persona, ProfessionalProfile, RoutineSuggestion,
    UserProfile,
};
pub use store::{MemoryStore, StoreError};
