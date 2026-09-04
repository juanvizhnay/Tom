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

pub fn persist_api_key<V: CredentialVault>(
    vault: &V,
    provider_id: &str,
    new_api_key: Option<&str>,
) -> Result<bool, CredentialError> {
    if let Some(api_key) = new_api_key {
        vault.set_key(provider_id, api_key)?;
        Ok(true)
    } else {
        vault.has_key(provider_id)
    }
}

pub fn clear_api_key<V: CredentialVault>(
    vault: &V,
    provider_id: &str,
) -> Result<bool, CredentialError> {
    vault.clear_key(provider_id)?;
    Ok(false)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashSet};

    use super::{CredentialError, CredentialVault, SERVICE_NAME, clear_api_key, persist_api_key};

    #[derive(Default)]
    struct FakeVault {
        keys: RefCell<HashSet<String>>,
        writes: RefCell<Vec<(String, String)>>,
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
            Ok(())
        }
    }

    #[test]
    fn credential_service_name_is_stable() {
        assert_eq!(SERVICE_NAME, "com.tom.assistant.ai");
    }

    #[test]
    fn an_empty_submission_preserves_the_existing_key_without_writing() {
        let vault = FakeVault::default();
        vault.keys.borrow_mut().insert("openai".into());

        let has_key = persist_api_key(&vault, "openai", None).expect("lookup should work");

        assert!(has_key);
        assert!(vault.writes.borrow().is_empty());
    }

    #[test]
    fn a_new_key_is_written_under_the_provider_id() {
        let vault = FakeVault::default();

        let has_key =
            persist_api_key(&vault, "gemini", Some("secret-value")).expect("write should work");

        assert!(has_key);
        assert_eq!(
            vault.writes.into_inner(),
            vec![("gemini".into(), "secret-value".into())]
        );
    }

    #[test]
    fn clearing_a_key_is_idempotent_and_reports_no_key() {
        let vault = FakeVault::default();
        vault.keys.borrow_mut().insert("anthropic".into());

        assert!(!clear_api_key(&vault, "anthropic").expect("delete should work"));
        assert!(!clear_api_key(&vault, "anthropic").expect("repeat delete should work"));
    }
}
