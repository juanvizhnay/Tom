//! Turns the assistant's catalog into the models the Slint interface renders.
//!
//! The interface owns layout and color; everything it *says* is built here from
//! `tom-core`, so a label only exists once and every id an interface callback hands
//! back is an id the domain already knows how to parse.

use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};
use tom_core::{
    Choice, OnboardingStep, PulseEntry, cloud_provider_choices, local_provider_choices,
    pulse_timeline, welcome_highlights,
};

use crate::{
    ApiKeyRow, ChoiceRow, MainWindow, OnboardingPage, OrbWindow, PulseRow,
    controller::{MAX_ORB_SIZE, MIN_ORB_SIZE},
    credential::ApiKeyStatus,
};

/// Publishes every fixed list and every piece of copy the dashboard draws.
pub fn bind_catalog(ui: &MainWindow) {
    ui.set_onboarding_pages(onboarding_pages());
    ui.set_welcome_highlights(welcome_highlight_rows());
    ui.set_persona_choices(step_choices(OnboardingStep::PersonaChoice));
    ui.set_profession_choices(step_choices(OnboardingStep::ProfessionChoice));
    ui.set_assistance_choices(step_choices(OnboardingStep::AssistanceChoice));
    ui.set_intelligence_choices(step_choices(OnboardingStep::IntelligenceChoice));
    ui.set_cloud_providers(choice_rows(&cloud_provider_choices()));
    ui.set_local_providers(choice_rows(&local_provider_choices()));
    ui.set_pulse_steps(pulse_rows());
}

/// Hands the orb window the size limits the controller enforces.
pub fn bind_orb_bounds(orb_ui: &OrbWindow) {
    orb_ui.set_orb_min_size(i32::from(MIN_ORB_SIZE));
    orb_ui.set_orb_max_size(i32::from(MAX_ORB_SIZE));
}

/// The copy for each onboarding question, in the order they are asked.
#[must_use]
pub fn onboarding_pages() -> ModelRc<OnboardingPage> {
    into_model(
        OnboardingStep::ALL
            .into_iter()
            .map(|step| OnboardingPage {
                rail_label: step.rail_label().into(),
                headline: step.headline().into(),
                body: step.body().into(),
                caption: step.orb_caption().into(),
                primary_label: step.primary_label().into(),
            })
            .collect(),
    )
}

/// One row of the key manager: a provider and whether it holds a key. No row is special.
#[must_use]
pub fn api_key_rows(inventory: &[ApiKeyStatus]) -> ModelRc<ApiKeyRow> {
    into_model(
        inventory
            .iter()
            .map(|status| ApiKeyRow {
                provider_id: status.provider.stable_id().into(),
                label: status.provider.label().into(),
                detail: status.provider.detail().into(),
                stored: status.stored,
            })
            .collect(),
    )
}

/// The options a single onboarding question offers.
#[must_use]
pub fn step_choices(step: OnboardingStep) -> ModelRc<ChoiceRow> {
    choice_rows(&step.choices())
}

/// The promises listed on the welcome step, numbered as they are shown.
#[must_use]
pub fn welcome_highlight_rows() -> ModelRc<SharedString> {
    into_model(
        welcome_highlights()
            .into_iter()
            .enumerate()
            .map(|(index, highlight)| format!("{:02}   {highlight}", index + 1).into())
            .collect(),
    )
}

/// The observed / inferred / proposed ladder, shared by Today and Routines.
#[must_use]
pub fn pulse_rows() -> ModelRc<PulseRow> {
    into_model(pulse_timeline().into_iter().map(pulse_row).collect())
}

fn pulse_row(entry: PulseEntry) -> PulseRow {
    PulseRow {
        stage: entry.tone.stage_label().into(),
        title: entry.title.into(),
        detail: entry.detail.into(),
        time: entry.time.into(),
        tone_id: entry.tone.stable_id().into(),
    }
}

#[must_use]
pub fn choice_rows(choices: &[Choice]) -> ModelRc<ChoiceRow> {
    into_model(
        choices
            .iter()
            .map(|choice| ChoiceRow {
                id: choice.id.into(),
                label: choice.label.into(),
                detail: choice.detail.into(),
            })
            .collect(),
    )
}

fn into_model<T: Clone + 'static>(rows: Vec<T>) -> ModelRc<T> {
    Rc::new(VecModel::from(rows)).into()
}

#[cfg(test)]
mod tests {
    use super::{
        api_key_rows, choice_rows, onboarding_pages, pulse_rows, step_choices,
        welcome_highlight_rows,
    };
    use crate::credential::ApiKeyStatus;
    use slint::Model;
    use tom_core::{
        AiProvider, OnboardingStep, api_key_providers, cloud_provider_choices,
        local_provider_choices, pulse_timeline, welcome_highlights,
    };

    #[test]
    fn every_onboarding_question_is_published_with_its_copy() {
        let pages = onboarding_pages();

        assert_eq!(pages.row_count(), OnboardingStep::ALL.len());
        for (index, step) in OnboardingStep::ALL.into_iter().enumerate() {
            let page = pages.row_data(index).expect("a page per step");
            assert_eq!(page.rail_label, step.rail_label());
            assert_eq!(page.headline, step.headline());
            assert_eq!(page.primary_label, step.primary_label());
            assert!(!page.body.is_empty());
            assert!(!page.caption.is_empty());
        }
    }

    #[test]
    fn choice_models_carry_the_ids_the_callbacks_will_send_back() {
        let choices = OnboardingStep::ProfessionChoice.choices();
        let rows = step_choices(OnboardingStep::ProfessionChoice);

        assert_eq!(rows.row_count(), choices.len());
        for (index, choice) in choices.iter().enumerate() {
            let row = rows.row_data(index).expect("a row per choice");
            assert_eq!(row.id, choice.id);
            assert_eq!(row.label, choice.label);
            assert_eq!(row.detail, choice.detail);
        }
    }

    #[test]
    fn provider_models_only_offer_ids_the_domain_can_resolve() {
        let cloud = choice_rows(&cloud_provider_choices());
        let local = choice_rows(&local_provider_choices());

        assert!(cloud.row_count() > 0 && local.row_count() > 0);
        for row in cloud.iter().chain(local.iter()) {
            let provider =
                AiProvider::from_stable_id(&row.id).expect("published ids stay resolvable");
            assert_ne!(provider, AiProvider::Disabled);
            assert!(!row.label.is_empty());
        }
    }

    #[test]
    fn the_key_manager_shows_one_row_per_provider_with_its_stored_state() {
        let inventory = api_key_providers()
            .into_iter()
            .map(|provider| ApiKeyStatus {
                provider,
                stored: provider == AiProvider::Anthropic,
            })
            .collect::<Vec<_>>();

        let rows = api_key_rows(&inventory);

        assert_eq!(rows.row_count(), inventory.len());
        for (index, status) in inventory.iter().enumerate() {
            let row = rows.row_data(index).expect("a row per provider");
            assert_eq!(row.provider_id, status.provider.stable_id());
            assert_eq!(row.label, status.provider.label());
            assert_eq!(row.stored, status.stored);
        }
        assert_eq!(
            rows.iter().filter(|row| row.stored).count(),
            1,
            "only the provider holding a key is marked as stored"
        );
    }

    #[test]
    fn welcome_highlights_are_numbered_in_order() {
        let rows = welcome_highlight_rows();

        assert_eq!(rows.row_count(), welcome_highlights().len());
        for (index, highlight) in welcome_highlights().into_iter().enumerate() {
            let row = rows.row_data(index).expect("a row per highlight");
            assert!(row.starts_with(&format!("{:02}", index + 1)));
            assert!(row.ends_with(highlight));
        }
    }

    #[test]
    fn the_pulse_model_mirrors_the_shared_timeline() {
        let rows = pulse_rows();
        let timeline = pulse_timeline();

        assert_eq!(rows.row_count(), timeline.len());
        for (index, entry) in timeline.into_iter().enumerate() {
            let row = rows.row_data(index).expect("a row per pulse entry");
            assert_eq!(row.stage, entry.tone.stage_label());
            assert_eq!(row.tone_id, entry.tone.stable_id());
            assert_eq!(row.title, entry.title);
            assert_eq!(row.detail, entry.detail);
        }
    }
}
