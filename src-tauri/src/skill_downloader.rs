use crate::credential_store;
use crate::gitlab_client;
use crate::models::{
    AppError, RepoRef, default_github_host, default_github_provider,
};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn download_repo(owner: &str, name: &str, branch: &str) -> Result<PathBuf, AppError> {
    download_repo_ref(&RepoRef {
        host: default_github_host(),
        provider: default_github_provider(),
        project_path: format!("{owner}/{name}"),
        branch: branch.to_string(),
    })
}

pub fn download_repo_ref(repo: &RepoRef) -> Result<PathBuf, AppError> {
    if repo.provider == "gitlab" {
        download_gitlab_repo(repo)
    } else {
        let (owner, name) = parse_github_project_path(&repo.project_path)?;
        download_github_repo(owner, name, &repo.branch)
    }
}

fn download_github_repo(owner: &str, name: &str, branch: &str) -> Result<PathBuf, AppError> {
    let primary_url = github_archive_url(owner, name, branch);
    match download_and_extract(&primary_url) {
        Ok(path) => Ok(path),
        Err(AppError::DownloadFailed { status: Some(404), .. }) => {
            let fallbacks = fallback_branches(branch);
            let mut last_error = AppError::DownloadFailed {
                url: primary_url,
                status: Some(404),
                message: format!(
                    "仓库 {}/{} 的分支 '{}' 不存在",
                    owner, name, branch
                ),
            };

            for fallback in fallbacks {
                let url = github_archive_url(owner, name, fallback);
                match download_and_extract(&url) {
                    Ok(path) => return Ok(path),
                    Err(err) => last_error = err,
                }
            }

            Err(last_error)
        }
        Err(err) => Err(err),
    }
}

fn download_gitlab_repo(repo: &RepoRef) -> Result<PathBuf, AppError> {
    let token = credential_store::get_gitlab_token(&repo.host)?;
    let Some(token) = token else {
        return Err(AppError::GitLabAuthRequired {
            host: repo.host.clone(),
        });
    };

    match gitlab_client::download_archive(
        &repo.host,
        &repo.project_path,
        &repo.branch,
        Some(&token),
    ) {
        Ok(path) => Ok(path),
        Err(err) if is_not_found_error(&err) => {
            let mut last_error = err;
            for fallback in fallback_branches(&repo.branch) {
                match gitlab_client::download_archive(
                    &repo.host,
                    &repo.project_path,
                    fallback,
                    Some(&token),
                ) {
                    Ok(path) => return Ok(path),
                    Err(fallback_err) => last_error = fallback_err,
                }
            }
            Err(last_error)
        }
        Err(err) => Err(err),
    }
}

fn is_not_found_error(err: &AppError) -> bool {
    matches!(
        err,
        AppError::SkillRepoNotFound { .. }
            | AppError::DownloadFailed {
                status: Some(404),
                ..
            }
    )
}

fn parse_github_project_path(project_path: &str) -> Result<(&str, &str), AppError> {
    project_path.split_once('/').ok_or_else(|| AppError::InvalidInput {
        input: project_path.to_string(),
        message: "GitHub 项目路径必须为 owner/name 格式".to_string(),
    })
}

pub fn download_and_extract(url: &str) -> Result<PathBuf, AppError> {
    download_and_extract_with_headers(url, &[])
}

pub fn download_and_extract_with_headers(
    url: &str,
    headers: &[(&str, &str)],
) -> Result<PathBuf, AppError> {
    let client = blocking_http_client();
    let mut request = client.get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }

    let response = request.send().map_err(|err| AppError::Io {
        path: None,
        message: format!("下载失败 {}: {}", url, err),
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(AppError::DownloadFailed {
            url: url.to_string(),
            status: Some(status.as_u16()),
            message: format!("下载失败，HTTP 状态码 {}", status.as_u16()),
        });
    }

    let bytes = response.bytes().map_err(|err| AppError::Io {
        path: None,
        message: format!("读取下载内容失败 {}: {}", url, err),
    })?;

    let extract_root = create_temp_extract_dir()?;

    extract_zip_bytes(&bytes, &extract_root)?;

    let extracted_dir = find_single_top_level_dir(&extract_root).ok_or_else(|| AppError::Io {
        path: Some(extract_root.clone()),
        message: "压缩包内未找到顶层目录".to_string(),
    })?;

    Ok(extracted_dir)
}

const HTTP_TIMEOUT_SECS: u64 = 60;

fn blocking_http_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

fn create_temp_extract_dir() -> Result<PathBuf, AppError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("创建临时目录失败: {}", err),
        })?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skills-sync-download-{}", nanos));
    fs::create_dir_all(&dir).map_err(|err| AppError::Io {
        path: Some(dir.clone()),
        message: format!("创建临时目录失败: {}", err),
    })?;
    Ok(dir)
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    if !src.is_dir() {
        return Err(AppError::Io {
            path: Some(src.to_path_buf()),
            message: "源路径不是目录".to_string(),
        });
    }

    fs::create_dir_all(dst).map_err(|err| io_error(Some(dst), err.to_string()))?;

    for entry in fs::read_dir(src).map_err(|err| io_error(Some(src), err.to_string()))? {
        let entry = entry.map_err(|err| io_error(Some(src), err.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|err| io_error(Some(entry.path()), err.to_string()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_symlink() {
            copy_symlink_as_file(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|err| io_error(Some(dst_path.clone()), err.to_string()))?;
        }
    }

    Ok(())
}

pub fn resolve_symlinks_in_dir(dir: &Path) -> Result<(), AppError> {
    if !dir.is_dir() {
        return Err(AppError::Io {
            path: Some(dir.to_path_buf()),
            message: "路径不是目录".to_string(),
        });
    }

    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).map_err(|err| io_error(Some(&current), err.to_string()))?
        {
            let entry = entry.map_err(|err| io_error(Some(&current), err.to_string()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|err| io_error(Some(path.clone()), err.to_string()))?;

            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_symlink() {
                replace_symlink_with_file(&path)?;
            }
        }
    }

    Ok(())
}

fn github_archive_url(owner: &str, name: &str, branch: &str) -> String {
    format!(
        "https://github.com/{}/{}/archive/refs/heads/{}.zip",
        owner, name, branch
    )
}

fn fallback_branches(requested: &str) -> Vec<&'static str> {
    let mut branches = Vec::new();
    if requested != "main" {
        branches.push("main");
    }
    if requested != "master" {
        branches.push("master");
    }
    branches
}

fn extract_zip_bytes(bytes: &[u8], dest: &Path) -> Result<(), AppError> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|err| AppError::Io {
        path: Some(dest.to_path_buf()),
        message: format!("无法读取 zip 压缩包: {}", err),
    })?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|err| AppError::Io {
            path: Some(dest.to_path_buf()),
            message: format!("无法读取 zip 条目: {}", err),
        })?;
        let Some(safe_path) = sanitize_zip_entry_path(file.name()) else {
            continue;
        };
        let out_path = dest.join(safe_path);

        if file.name().ends_with('/') {
            fs::create_dir_all(&out_path)
                .map_err(|err| io_error(Some(&out_path), err.to_string()))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|err| io_error(Some(parent), err.to_string()))?;
        }

        let mut out_file = fs::File::create(&out_path)
            .map_err(|err| io_error(Some(&out_path), err.to_string()))?;
        std::io::copy(&mut file, &mut out_file)
            .map_err(|err| io_error(Some(&out_path), err.to_string()))?;
    }

    Ok(())
}

fn sanitize_zip_entry_path(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path
        .components()
        .any(|component| matches!(component, Component::Normal(part) if part == ".."))
    {
        return None;
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }

    if safe.as_os_str().is_empty() {
        None
    } else {
        Some(safe)
    }
}

fn find_single_top_level_dir(root: &Path) -> Option<PathBuf> {
    let mut dirs = fs::read_dir(root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    if dirs.len() == 1 {
        dirs.pop()
    } else {
        None
    }
}

fn copy_symlink_as_file(src: &Path, dst: &Path) -> Result<(), AppError> {
    let target = fs::read_link(src).map_err(|err| io_error(Some(src), err.to_string()))?;
    let resolved = if target.is_absolute() {
        target
    } else {
        src.parent()
            .map(|parent| parent.join(&target))
            .unwrap_or(target)
    };

    if resolved.is_dir() {
        copy_dir_recursive(&resolved, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|err| io_error(Some(parent), err.to_string()))?;
        }
        fs::copy(&resolved, dst).map_err(|err| io_error(Some(dst), err.to_string()))?;
        Ok(())
    }
}

fn replace_symlink_with_file(path: &Path) -> Result<(), AppError> {
    let target = fs::read_link(path).map_err(|err| io_error(Some(path), err.to_string()))?;
    let resolved = if target.is_absolute() {
        target
    } else {
        path.parent()
            .map(|parent| parent.join(&target))
            .unwrap_or(target)
    };

    let temp_path = path.with_extension("symlink-resolving");
    if resolved.is_dir() {
        copy_dir_recursive(&resolved, &temp_path)?;
    } else {
        if let Some(parent) = temp_path.parent() {
            fs::create_dir_all(parent).map_err(|err| io_error(Some(parent), err.to_string()))?;
        }
        fs::copy(&resolved, &temp_path).map_err(|err| io_error(Some(&temp_path), err.to_string()))?;
    }

    fs::remove_file(path).map_err(|err| io_error(Some(path), err.to_string()))?;
    if resolved.is_dir() {
        fs::rename(&temp_path, path).map_err(|err| io_error(Some(path), err.to_string()))?;
    } else {
        fs::rename(&temp_path, path).map_err(|err| io_error(Some(path), err.to_string()))?;
    }

    Ok(())
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
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_zip(root_name: &str, entries: &[(&str, &str)]) -> Vec<u8> {
        let buffer = Vec::new();
        let cursor = std::io::Cursor::new(buffer);
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();

        for (relative_path, content) in entries {
            let path = format!("{}/{}", root_name, relative_path);
            writer
                .start_file(&path, options)
                .expect("start zip file");
            writer
                .write_all(content.as_bytes())
                .expect("write zip content");
        }

        writer.finish().expect("finish zip").into_inner()
    }

    #[test]
    fn download_and_extract_returns_root_folder_with_skill_md() {
        let zip_bytes = create_test_zip("foo-main", &[("foo/SKILL.md", "# Skill")]);
        let temp = tempfile::tempdir().expect("tempdir");
        let zip_path = temp.path().join("fixture.zip");
        fs::write(&zip_path, zip_bytes).expect("write zip");

        let extract_root = temp.path().join("extract");
        fs::create_dir_all(&extract_root).expect("create extract dir");
        extract_zip_bytes(&fs::read(&zip_path).unwrap(), &extract_root).expect("extract");

        let extracted = find_single_top_level_dir(&extract_root).expect("top-level dir");
        assert_eq!(extracted.file_name().unwrap().to_string_lossy(), "foo-main");
        assert_eq!(
            fs::read_to_string(extracted.join("foo").join("SKILL.md")).unwrap(),
            "# Skill"
        );
    }

    #[test]
    fn copy_dir_recursive_copies_nested_structure() {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        let nested = src.join("nested");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(src.join("root.txt"), "root").expect("write root");
        fs::write(nested.join("child.txt"), "child").expect("write child");

        let dst = temp.path().join("dst");
        copy_dir_recursive(&src, &dst).expect("copy");

        assert_eq!(fs::read_to_string(dst.join("root.txt")).unwrap(), "root");
        assert_eq!(
            fs::read_to_string(dst.join("nested").join("child.txt")).unwrap(),
            "child"
        );
    }

    #[cfg(unix)]
    #[test]
    fn resolve_symlinks_in_dir_replaces_symlink_with_file_contents() {
        let temp = tempfile::tempdir().expect("tempdir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        std::os::unix::fs::symlink(&target, &link).expect("create symlink");

        resolve_symlinks_in_dir(temp.path()).expect("resolve symlinks");

        assert!(link.is_file());
        assert!(!link.is_symlink());
        assert_eq!(fs::read_to_string(&link).unwrap(), "payload");
    }

    #[test]
    fn sanitize_zip_entry_path_rejects_parent_traversal() {
        assert!(sanitize_zip_entry_path("../escape.txt").is_none());
        assert!(sanitize_zip_entry_path("safe/path.txt").is_some());
    }
}
