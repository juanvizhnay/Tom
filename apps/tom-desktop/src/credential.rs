use tom_core::{AiProvider, api_key_providers};

pub const SERVICE_NAME: &str = "com.tom.assistant.ai";

#[derive(Debug)]
pub struct CredentialError;

pub trait CredentialVault {
    fn has_key(&self, provider_id: &str) -> Result<bool, CredentialError>;
    fn set_key(&self, provider_id: &str, api_key: &str) -> Result<(), CredentialError>;
    fn clear_key(&self, provider_id: &str) -> Result<(), CredentialError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsCredentialVault;

impl WindowsCredentialVault {
    fn entry(provider_id: &str) -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new(SERVICE_NAME, provider_id).map_err(|_| CredentialError)
    }
}

impl CredentialVault for WindowsCredentialVault {
    fn has_key(&self, provider_id: &str) -> Result<bool, CredentialError> {
        match Self::entry(provider_id)?.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(_) => Err(CredentialError),
        }
    }

    fn set_key(&self, provider_id: &str, api_key: &str) -> Result<(), CredentialError> {
        Self::entry(provider_id)?
            .set_password(api_key)
            .map_err(|_| CredentialError)
    }

    fn clear_key(&self, provider_id: &str) -> Result<(), CredentialError> {
        match Self::entry(provider_id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(CredentialError),
        }
    }
}

/// Whether one provider currently holds a credential.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiKeyStatus {
    pub provider: AiProvider,
    pub stored: bool,
}

/// Reads which providers hold a credential right now.
///
/// Keys live one per provider, so this is the only place that decides what the manager
/// shows as saved: nothing is inferred from the provider that happens to be selected.
pub fn read_key_inventory<V: CredentialVault>(
    vault: &V,
) -> Result<Vec<ApiKeyStatus>, CredentialError> {
    api_key_providers()
        .into_iter()
        .map(|provider| {
            vault
                .has_key(provider.stable_id())
                .map(|stored| ApiKeyStatus { provider, stored })
        })
        .collect()
}

/// Writes one provider's credential, replacing only that provider's key.
///
/// Adding a second provider never disturbs the first: the vault is keyed by provider id.
pub fn store_api_key<V: CredentialVault>(
    vault: &V,
    provider_id: &str,
    api_key: &str,
) -> Result<(), CredentialError> {
    vault.set_key(provider_id, api_key)
}

/// Removes one provider's credential, and only when the user asks for it.
///
/// Deleting a key that is not there succeeds, so a stale interface cannot raise an error.
pub fn forget_api_key<V: CredentialVault>(
    vault: &V,
    provider_id: &str,
) -> Result<(), CredentialError> {
    vault.clear_key(provider_id)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashSet};

    use super::{
        ApiKeyStatus, CredentialError, CredentialVault, SERVICE_NAME, forget_api_key,
        read_key_inventory, store_api_key,
    };
    use tom_core::{AiProvider, api_key_providers};

    #[derive(Default)]
    struct FakeVault {
        keys: RefCell<HashSet<String>>,
        writes: RefCell<Vec<(String, String)>>,
        deletes: RefCell<Vec<String>>,
    }

    impl FakeVault {
        fn with_keys(provider_ids: &[&str]) -> Self {
            let vault = Self::default();
            for provider_id in provider_ids {
                vault.keys.borrow_mut().insert((*provider_id).to_owned());
            }
            vault
        }
    }

    impl CredentialVault for FakeVault {
        fn has_key(&self, provider_id: &str) -> Result<bool, CredentialError> {
            Ok(self.keys.borrow().contains(provider_id))
        }

        fn set_key(&self, provider_id: &str, api_key: &str) -> Result<(), CredentialError> {
            self.keys.borrow_mut().insert(provider_id.to_owned());
            self.writes
                .borrow_mut()
                .push((provider_id.to_owned(), api_key.to_owned()));
            Ok(())
        }

        fn clear_key(&self, provider_id: &str) -> Result<(), CredentialError> {
            self.keys.borrow_mut().remove(provider_id);
            self.deletes.borrow_mut().push(provider_id.to_owned());
            Ok(())
        }
    }

    #[test]
    fn credential_service_name_is_stable() {
        assert_eq!(SERVICE_NAME, "com.tom.assistant.ai");
    }

    #[test]
    fn adding_a_key_leaves_every_other_provider_untouched() {
        let vault = FakeVault::with_keys(&["openai"]);

        store_api_key(&vault, "anthropic", "anthropic-secret").expect("write should work");

        let inventory = read_key_inventory(&vault).expect("inventory should work");
        let stored = stored_providers(&inventory);
        assert!(stored.contains(&AiProvider::OpenAi));
        assert!(stored.contains(&AiProvider::Anthropic));
        assert_eq!(stored.len(), 2);
        assert!(vault.deletes.borrow().is_empty());
    }

    #[test]
    fn the_inventory_covers_every_provider_that_accepts_a_key() {
        let vault = FakeVault::with_keys(&["gemini", "lm-studio"]);

        let inventory = read_key_inventory(&vault).expect("inventory should work");

        assert_eq!(inventory.len(), api_key_providers().len());
        assert_eq!(
            stored_providers(&inventory),
            vec![AiProvider::Gemini, AiProvider::LmStudio]
        );
        assert!(
            !inventory
                .iter()
                .any(|status| status.provider == AiProvider::Disabled)
        );
    }

    #[test]
    fn forgetting_a_key_removes_only_that_provider_and_repeats_cleanly() {
        let vault = FakeVault::with_keys(&["openai", "anthropic"]);

        forget_api_key(&vault, "anthropic").expect("delete should work");
        forget_api_key(&vault, "anthropic").expect("repeat delete should work");

        let inventory = read_key_inventory(&vault).expect("inventory should work");
        assert_eq!(stored_providers(&inventory), vec![AiProvider::OpenAi]);
    }

    #[test]
    fn a_stored_key_is_written_verbatim_under_its_provider_id() {
        let vault = FakeVault::default();

        store_api_key(&vault, "custom-local", "local-token").expect("write should work");

        assert_eq!(
            vault.writes.into_inner(),
            vec![("custom-local".into(), "local-token".into())]
        );
    }

    fn stored_providers(inventory: &[ApiKeyStatus]) -> Vec<AiProvider> {
        inventory
            .iter()
            .filter(|status| status.stored)
            .map(|status| status.provider)
            .collect()
    }
}
