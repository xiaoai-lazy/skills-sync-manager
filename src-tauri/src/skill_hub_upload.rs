use crate::models::{AppConfig, AppError, DiscoverableSkill, SkillRecord};
use crate::skill_hub_client;
use crate::skill_hub_discover;
use crate::skill_hub_endpoints;
use crate::skill_library;
use crate::skill_storage;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub fn upload_skill_to_hub(
    config: &mut AppConfig,
    hub_endpoint_id: &str,
    group: &str,
    storage_key: &str,
    main_dir: &Path,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    if storage_key.trim().is_empty() {
        return Err(AppError::InvalidInput {
            input: storage_key.to_string(),
            message: "storageKey 不能为空".to_string(),
        });
    }

    let skill_dir = skill_storage::main_library_path(main_dir, storage_key);
    if !skill_dir.is_dir() {
        return Err(AppError::SkillDirNotFound {
            path: skill_dir,
        });
    }
    if !skill_dir.join("SKILL.md").is_file() {
        return Err(AppError::SkillDirNotFound {
            path: skill_dir.join("SKILL.md"),
        });
    }

    let skill_id = resolve_upload_skill_id(config, storage_key, &skill_dir)?;
    let base_url = skill_hub_endpoints::hub_endpoint_base_url(config, hub_endpoint_id)?;
    let archive_path = zip_skill_directory(&skill_dir)?;

    let upload_result = skill_hub_client::upload_skill(&base_url, group, &skill_id, &archive_path);
    let _ = fs::remove_file(&archive_path);
    upload_result?;

    skill_hub_discover::merge_hub_endpoint_into_discover_cache(config, hub_endpoint_id)?;

    Ok(hub_discover_skills_from_cache(config, hub_endpoint_id))
}

fn skill_id_from_storage_key(storage_key: &str) -> Result<String, AppError> {
    storage_key
        .rsplit('/')
        .find(|part| !part.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AppError::InvalidInput {
            input: storage_key.to_string(),
            message: "storageKey 无效".to_string(),
        })
}

fn resolve_upload_skill_id(
    config: &AppConfig,
    storage_key: &str,
    skill_dir: &Path,
) -> Result<String, AppError> {
    if let Some(record) = config.skill_records.get(storage_key) {
        if let Some(skill_id) = upload_skill_id_from_record(record) {
            return Ok(skill_id);
        }
    }

    for record in config.skill_records.values() {
        if record.storage_key == storage_key {
            if let Some(skill_id) = upload_skill_id_from_record(record) {
                return Ok(skill_id);
            }
        }
    }

    let skill_md = skill_dir.join("SKILL.md");
    if skill_md.is_file() {
        let raw = fs::read_to_string(&skill_md).map_err(|err| io_error(Some(&skill_md), err.to_string()))?;
        if let Some(metadata) = skill_library::parse_valid_skill_metadata(&raw) {
            let skill_id = metadata.name.trim().to_string();
            if is_valid_hub_skill_id(&skill_id) {
                return Ok(skill_id);
            }
        }
    }

    skill_id_from_storage_key(storage_key)
}

fn upload_skill_id_from_record(record: &SkillRecord) -> Option<String> {
    if !record.link_name.is_empty() {
        return Some(record.link_name.clone());
    }
    if record.source == "skillhub" && !record.hub_skill_id.is_empty() {
        return Some(record.hub_skill_id.clone());
    }
    None
}

fn is_valid_hub_skill_id(skill_id: &str) -> bool {
    !skill_id.is_empty()
        && !skill_id.contains('/')
        && !skill_id.contains('\\')
        && !skill_id.contains("..")
}

pub fn skill_id_from_hub_storage_key(storage_key: &str) -> Result<String, AppError> {
    skill_id_from_storage_key(storage_key)
}

fn hub_discover_skills_from_cache(
    config: &AppConfig,
    hub_endpoint_id: &str,
) -> Vec<DiscoverableSkill> {
    config
        .skill_discover_cache
        .skills
        .iter()
        .filter(|skill| {
            skill.source == "skillhub" && skill.hub_endpoint_id == hub_endpoint_id
        })
        .cloned()
        .collect()
}

pub fn zip_skill_directory(skill_dir: &Path) -> Result<PathBuf, AppError> {
    let zip_path = create_temp_upload_zip_path()?;
    let file = File::create(&zip_path).map_err(|err| io_error(Some(&zip_path), err.to_string()))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_directory_to_zip(&mut writer, skill_dir, skill_dir, options)?;
    writer
        .finish()
        .map_err(|err| io_error(Some(&zip_path), err.to_string()))?;

    Ok(zip_path)
}

fn add_directory_to_zip(
    writer: &mut ZipWriter<File>,
    base_dir: &Path,
    current_dir: &Path,
    options: SimpleFileOptions,
) -> Result<(), AppError> {
    for entry in fs::read_dir(current_dir).map_err(|err| io_error(Some(current_dir), err.to_string()))?
    {
        let entry = entry.map_err(|err| io_error(Some(current_dir), err.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            add_directory_to_zip(writer, base_dir, &path, options)?;
            continue;
        }

        let relative = path
            .strip_prefix(base_dir)
            .map_err(|_| AppError::Io {
                path: Some(path.clone()),
                message: "无法计算 zip 相对路径".to_string(),
            })?
            .to_string_lossy()
            .replace('\\', "/");

        writer
            .start_file(&relative, options)
            .map_err(|err| io_error(Some(&path), err.to_string()))?;
        let mut file =
            File::open(&path).map_err(|err| io_error(Some(&path), err.to_string()))?;
        let mut buffer = [0u8; 8192];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|err| io_error(Some(&path), err.to_string()))?;
            if read == 0 {
                break;
            }
            writer
                .write_all(&buffer[..read])
                .map_err(|err| io_error(Some(&path), err.to_string()))?;
        }
    }

    Ok(())
}

fn create_temp_upload_zip_path() -> Result<PathBuf, AppError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("创建临时 zip 失败: {}", err),
        })?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("skills-sync-hub-upload-{}.zip", nanos)))
}

fn io_error(path: Option<impl AsRef<Path>>, message: String) -> AppError {
    AppError::Io {
        path: path.map(|value| value.as_ref().to_path_buf()),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, SkillRecord};
    use std::collections::HashMap;
    use std::fs;

    fn write_valid_skill(dir: &Path, name: &str) {
        fs::create_dir_all(dir).expect("create skill dir");
        fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill\n",
                name
            ),
        )
        .expect("write skill md");
    }

    #[test]
    fn resolve_upload_skill_id_prefers_record_link_name_over_storage_key_tail() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_dir = temp.path().join("skill");
        write_valid_skill(&skill_dir, "kqs-review");

        let storage_key = "repo/git.example.com--team/tools/kqs-review-upload-test";
        let mut records = HashMap::new();
        records.insert(
            storage_key.to_string(),
            SkillRecord {
                link_name: "kqs-review".to_string(),
                storage_key: storage_key.to_string(),
                ..Default::default()
            },
        );

        let config = AppConfig {
            skill_records: records,
            ..Default::default()
        };

        assert_eq!(
            resolve_upload_skill_id(&config, storage_key, &skill_dir).unwrap(),
            "kqs-review"
        );
    }

    #[test]
    fn resolve_upload_skill_id_uses_skill_md_name_when_record_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_dir = temp.path().join("skill");
        write_valid_skill(&skill_dir, "kqs-review");

        let config = AppConfig::default();
        let storage_key = "repo/git.example.com--team/tools/kqs-review-upload-test";

        assert_eq!(
            resolve_upload_skill_id(&config, storage_key, &skill_dir).unwrap(),
            "kqs-review"
        );
    }

    #[test]
    fn skill_id_from_storage_key_accepts_repo_path() {
        assert_eq!(
            skill_id_from_storage_key(
                "repo/git.xkw.cn--mp-oxygen-uc-skills/talos-lecture-json-review"
            )
            .unwrap(),
            "talos-lecture-json-review"
        );
    }

    #[test]
    fn skill_id_from_hub_storage_key_returns_last_segment() {
        assert_eq!(
            skill_id_from_hub_storage_key("hub/company-hub/common/tdd").unwrap(),
            "tdd"
        );
    }

    #[test]
    fn zip_skill_directory_includes_skill_md() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_dir = temp.path().join("skill");
        write_valid_skill(&skill_dir, "tdd");
        fs::write(skill_dir.join("extra.txt"), "payload").expect("write extra");

        let zip_path = zip_skill_directory(&skill_dir).expect("zip skill dir");
        assert!(zip_path.is_file());

        let extract_root = temp.path().join("extract");
        fs::create_dir_all(&extract_root).expect("create extract dir");
        crate::skill_downloader::extract_zip_file(&zip_path, &extract_root).expect("extract");

        assert!(extract_root.join("SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(extract_root.join("extra.txt")).unwrap(),
            "payload"
        );

        let _ = fs::remove_file(zip_path);
    }
}
