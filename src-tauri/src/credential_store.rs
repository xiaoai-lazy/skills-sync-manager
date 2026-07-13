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

fn unregister_gitlab_host_with<F>(
    config: &mut AppConfig,
    host: &str,
    delete_token: F,
) -> Result<(), AppError>
where
    F: FnOnce(&str) -> Result<(), AppError>,
{
    delete_token(host)?;
    let normalized = normalize_host(host);
    config
        .gitlab_credential_hosts
        .retain(|existing| existing != &normalized);
    Ok(())
}

pub fn unregister_gitlab_host(config: &mut AppConfig, host: &str) -> Result<(), AppError> {
    unregister_gitlab_host_with(config, host, remove_gitlab_token)
}

pub fn list_configured_gitlab_hosts(config: &AppConfig) -> Vec<String> {
    config.gitlab_credential_hosts.clone()
}

fn reconcile_gitlab_credential_hosts_with<F>(config: &mut AppConfig, get_token: F) -> bool
where
    F: Fn(&str) -> Result<Option<String>, AppError>,
{
    let mut changed = false;
    let mut gitlab_hosts = config
        .skill_repos
        .iter()
        .filter(|repo| repo.provider == "gitlab")
        .map(|repo| normalize_host(&repo.host))
        .filter(|host| !host.is_empty())
        .collect::<Vec<_>>();
    gitlab_hosts.sort();
    gitlab_hosts.dedup();

    for host in gitlab_hosts {
        match get_token(&host) {
            Ok(Some(_)) => {
                let before = config.gitlab_credential_hosts.len();
                register_gitlab_host(config, &host);
                changed |= config.gitlab_credential_hosts.len() > before;
            }
            Ok(None) => {
                let before = config.gitlab_credential_hosts.len();
                config
                    .gitlab_credential_hosts
                    .retain(|existing| existing != &host);
                changed |= config.gitlab_credential_hosts.len() < before;
            }
            Err(_) => {}
        }
    }

    changed
}

/// Keep configured GitLab credential hosts aligned with the OS credential store.
pub fn reconcile_gitlab_credential_hosts(config: &mut AppConfig) -> bool {
    reconcile_gitlab_credential_hosts_with(config, get_gitlab_token)
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
    fn reconcile_gitlab_credential_hosts_registers_hosts_with_tokens() {
        let host = "reconcile.example.test";
        let _ = remove_gitlab_token(host);
        set_gitlab_token(host, "glpat-reconcile").expect("set");

        let mut config = AppConfig::default();
        config.skill_repos.push(crate::models::SkillRepo {
            host: host.to_string(),
            provider: "gitlab".to_string(),
            project_path: "group/project".to_string(),
            owner: "group".to_string(),
            name: "project".to_string(),
            branch: "main".to_string(),
            enabled: true,
        });

        assert!(reconcile_gitlab_credential_hosts(&mut config));
        assert_eq!(config.gitlab_credential_hosts, vec![host.to_string()]);

        assert!(!reconcile_gitlab_credential_hosts(&mut config));
        remove_gitlab_token(host).expect("remove");
    }

    #[test]
    fn reconcile_gitlab_credential_hosts_skips_hosts_without_tokens() {
        let mut config = AppConfig::default();
        config.skill_repos.push(crate::models::SkillRepo {
            host: "missing-token.example.test".to_string(),
            provider: "gitlab".to_string(),
            project_path: "group/project".to_string(),
            owner: "group".to_string(),
            name: "project".to_string(),
            branch: "main".to_string(),
            enabled: true,
        });

        assert!(!reconcile_gitlab_credential_hosts(&mut config));
        assert!(config.gitlab_credential_hosts.is_empty());
    }

    #[test]
    fn reconcile_gitlab_credential_hosts_removes_stale_config_without_token() {
        let host = "stale-token.example.test";
        let mut config = AppConfig::default();
        config.skill_repos.push(crate::models::SkillRepo {
            host: host.to_string(),
            provider: "gitlab".to_string(),
            project_path: "group/project".to_string(),
            owner: "group".to_string(),
            name: "project".to_string(),
            branch: "main".to_string(),
            enabled: true,
        });
        register_gitlab_host(&mut config, host);

        assert!(reconcile_gitlab_credential_hosts_with(&mut config, |_| Ok(None)));
        assert!(config.gitlab_credential_hosts.is_empty());
    }

    #[test]
    fn reconcile_gitlab_credential_hosts_keeps_state_when_keyring_read_fails() {
        let host = "keyring-error.example.test";
        let mut config = AppConfig::default();
        config.skill_repos.push(crate::models::SkillRepo {
            host: host.to_string(),
            provider: "gitlab".to_string(),
            project_path: "group/project".to_string(),
            owner: "group".to_string(),
            name: "project".to_string(),
            branch: "main".to_string(),
            enabled: true,
        });
        register_gitlab_host(&mut config, host);

        assert!(!reconcile_gitlab_credential_hosts_with(&mut config, |_| {
            Err(AppError::CredentialStore {
                message: "read failed".to_string(),
            })
        }));
        assert_eq!(config.gitlab_credential_hosts, vec![host.to_string()]);
    }

    #[test]
    fn unregister_gitlab_host_removes_config_after_token_delete_succeeds() {
        let host = "unregister.example.test";
        let mut config = AppConfig::default();
        register_gitlab_host(&mut config, host);

        let mut deleted = false;
        unregister_gitlab_host_with(&mut config, host, |_| {
            deleted = true;
            Ok(())
        })
        .expect("unregister");

        assert!(deleted);
        assert!(config.gitlab_credential_hosts.is_empty());
    }

    #[test]
    fn unregister_gitlab_host_keeps_config_when_token_delete_fails() {
        let host = "unregister.example.test";
        let mut config = AppConfig::default();
        register_gitlab_host(&mut config, host);

        let error = unregister_gitlab_host_with(&mut config, host, |_| {
            Err(AppError::CredentialStore {
                message: "delete failed".to_string(),
            })
        })
        .expect_err("delete should fail");

        assert!(matches!(error, AppError::CredentialStore { .. }));
        assert_eq!(config.gitlab_credential_hosts, vec![host.to_string()]);
    }
}
