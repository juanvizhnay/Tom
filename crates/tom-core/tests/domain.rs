use tom_core::{
    AssistanceStyle, Note, Persona, ProfessionalProfile, RoutineSuggestion, UserProfile,
};

#[test]
fn personas_expose_the_expected_display_names() {
    assert_eq!(Persona::Tom.display_name(), "Tom");
    assert_eq!(Persona::Tomy.display_name(), "Tomy");
}

#[test]
fn a_user_profile_defaults_to_professional_tom_for_developers() {
    let profile = UserProfile::default();

    assert_eq!(profile.persona, Persona::Tom);
    assert_eq!(profile.professional_profile, ProfessionalProfile::Developer);
    assert_eq!(profile.assistance_style, AssistanceStyle::Balanced);
}

#[test]
fn professional_profiles_and_assistance_styles_have_stable_ids() {
    let profiles = [
        ProfessionalProfile::Developer,
        ProfessionalProfile::Office,
        ProfessionalProfile::Student,
        ProfessionalProfile::Creator,
        ProfessionalProfile::General,
    ];
    for profile in profiles {
        assert_eq!(
            ProfessionalProfile::from_stable_id(profile.stable_id()),
            Some(profile)
        );
    }

    let styles = [
        AssistanceStyle::Focused,
        AssistanceStyle::Balanced,
        AssistanceStyle::Proactive,
    ];
    for style in styles {
        assert_eq!(
            AssistanceStyle::from_stable_id(style.stable_id()),
            Some(style)
        );
    }
}

#[test]
fn a_note_is_created_with_stable_identity_and_timestamps() {
    let note = Note::new("Stand-up", "Review the pull request");

    assert_eq!(note.title, "Stand-up");
    assert_eq!(note.body, "Review the pull request");
    assert_eq!(note.created_at, note.updated_at);
    assert!(!note.id.is_nil());
}

#[test]
fn a_routine_suggestion_captures_the_reason_and_proposed_action() {
    let suggestion = RoutineSuggestion::new(
        "Prepare stand-up",
        "You open the board every weekday at 08:55",
        "Open the board and draft yesterday's progress",
    );

    assert_eq!(suggestion.title, "Prepare stand-up");
    assert_eq!(
        suggestion.rationale,
        "You open the board every weekday at 08:55"
    );
    assert_eq!(
        suggestion.proposed_action,
        "Open the board and draft yesterday's progress"
    );
    assert!(!suggestion.id.is_nil());
}
