use crate::models::{AppConfig, AppError};
use keyring::Entry;

const SERVICE: &str = "skills-sync-manager";

pub fn gitlab_key(host: &str) -> String {
    format!("gitlab:{}", host.trim().to_lowercase())
}

fn entry_for_host(host: &str) -> Result<Entry, AppError> {
    Entry::new(SERVICE, &gitlab_key(host)).map_err(|error| AppError::CredentialStore {
        message: format!("无法创建凭证条目：{error}"),
    })
}

pub fn set_gitlab_token(host: &str, token: &str) -> Result<(), AppError> {
    entry_for_host(host)?
        .set_password(token)
        .map_err(|error| AppError::CredentialStore {
            message: format!("无法保存 GitLab 凭证：{error}"),
        })
}

pub fn get_gitlab_token(host: &str) -> Result<Option<String>, AppError> {
    match entry_for_host(host)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(AppError::CredentialStore {
            message: format!("无法读取 GitLab 凭证：{error}"),
        }),
    }
}

pub fn remove_gitlab_token(host: &str) -> Result<(), AppError> {
    match entry_for_host(host)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AppError::CredentialStore {
            message: format!("无法删除 GitLab 凭证：{error}"),
        }),
    }
}

fn normalize_host(host: &str) -> String {
    host.trim().to_lowercase()
}

pub fn register_gitlab_host(config: &mut AppConfig, host: &str) {
    let normalized = normalize_host(host);
    if normalized.is_empty() {
        return;
    }
    if !config
        .gitlab_credential_hosts
        .iter()
        .any(|existing| existing == &normalized)
    {
        config.gitlab_credential_hosts.push(normalized);
    }
}

pub fn unregister_gitlab_host(config: &mut AppConfig, host: &str) {
    let normalized = normalize_host(host);
    config
        .gitlab_credential_hosts
        .retain(|existing| existing != &normalized);
    let _ = remove_gitlab_token(host);
}

pub fn list_configured_gitlab_hosts(config: &AppConfig) -> Vec<String> {
    config.gitlab_credential_hosts.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitlab_key_normalizes_host() {
        assert_eq!(gitlab_key("  GitLab.Example.COM  "), "gitlab:gitlab.example.com");
    }

    #[test]
    fn credential_roundtrip() {
        let host = "git.example.test";
        let _ = remove_gitlab_token(host);
        set_gitlab_token(host, "glpat-test-token").expect("set");
        let got = get_gitlab_token(host).expect("get");
        assert_eq!(got.as_deref(), Some("glpat-test-token"));
        remove_gitlab_token(host).expect("remove");
        assert_eq!(get_gitlab_token(host).unwrap(), None);
    }

    #[test]
    fn register_gitlab_host_dedupes() {
        let mut config = AppConfig::default();
        register_gitlab_host(&mut config, "GitLab.Example.COM");
        register_gitlab_host(&mut config, "gitlab.example.com");
        register_gitlab_host(&mut config, "  gitlab.example.com  ");
        assert_eq!(
            config.gitlab_credential_hosts,
            vec!["gitlab.example.com".to_string()]
        );
        assert_eq!(
            list_configured_gitlab_hosts(&config),
            vec!["gitlab.example.com".to_string()]
        );
    }

    #[test]
    fn unregister_gitlab_host_removes_host_and_token() {
        let host = "unregister.example.test";
        let _ = remove_gitlab_token(host);
        set_gitlab_token(host, "glpat-unregister").expect("set");

        let mut config = AppConfig::default();
        register_gitlab_host(&mut config, host);
        assert_eq!(config.gitlab_credential_hosts.len(), 1);

        unregister_gitlab_host(&mut config, host);
        assert!(config.gitlab_credential_hosts.is_empty());
        assert_eq!(get_gitlab_token(host).unwrap(), None);
    }
}
