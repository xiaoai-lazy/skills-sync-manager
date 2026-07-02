use crate::models::{AppError, LinkType};
use std::fs;
use std::path::{Path, PathBuf};

pub fn default_link_type() -> LinkType {
    if cfg!(windows) {
        LinkType::Junction
    } else {
        LinkType::Symlink
    }
}

pub fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

pub fn create_dir_link(
    source: &Path,
    link_path: &Path,
    link_type: LinkType,
) -> Result<(), AppError> {
    match link_type {
        LinkType::Junction => create_junction(source, link_path),
        LinkType::Symlink => create_dir_symlink(source, link_path),
    }
}

pub fn link_target(path: &Path) -> Result<Option<PathBuf>, AppError> {
    if !is_link(path)? {
        return Ok(None);
    }

    fs::read_link(path)
        .map(Some)
        .map_err(|err| io_error(Some(path), format!("failed to read link target: {}", err)))
}

pub fn remove_recorded_link(link_path: &Path, expected_target: &Path) -> Result<(), AppError> {
    let Some(actual_target) = link_target(link_path)? else {
        return Err(io_error(
            Some(link_path),
            format!(
                "无法删除：{} 不是链接，软件不会删除未知内容",
                link_path.display()
            ),
        ));
    };

    if !same_target(&actual_target, expected_target) {
        return Err(io_error(
            Some(link_path),
            format!(
                "无法删除：{} 指向的目标与记录不符（期望 {}，实际 {}）",
                link_path.display(),
                expected_target.display(),
                actual_target.display()
            ),
        ));
    }

    if link_path.is_dir() {
        fs::remove_dir(link_path)
    } else {
        fs::remove_file(link_path)
    }
    .map_err(|err| io_error(Some(link_path), format!("failed to remove link: {}", err)))
}

pub fn delete_real_dir(path: &Path) -> Result<(), AppError> {
    if !path.is_dir() || is_link(path)? {
        return Err(io_error(
            Some(path),
            "refusing to recursively delete path because it is not a real directory".to_string(),
        ));
    }

    fs::remove_dir_all(path).map_err(|err| {
        io_error(
            Some(path),
            format!("failed to recursively delete directory: {}", err),
        )
    })
}

#[cfg(windows)]
fn create_junction(source: &Path, link_path: &Path) -> Result<(), AppError> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let source = windows_junction_path(source, false)?;
    let link_path = windows_junction_path(link_path, true)?;

    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link_path)
        .arg(&source)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| io_error(Some(&link_path), format!("failed to invoke mklink: {}", err)))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if stderr.is_empty() { stdout } else { stderr };
        Err(io_error(
            Some(&link_path),
            format!("failed to create junction: {}", message),
        ))
    }
}

/// Prepare an absolute path with backslashes for `cmd.exe mklink`.
#[cfg(windows)]
fn windows_junction_path(path: &Path, allow_missing_leaf: bool) -> Result<PathBuf, AppError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|err| {
                io_error(
                    Some(path),
                    format!("failed to resolve working directory: {}", err),
                )
            })?
            .join(path)
    };

    let normalized = if absolute.exists() {
        fs::canonicalize(&absolute).map_err(|err| {
            io_error(
                Some(path),
                format!("failed to canonicalize path: {}", err),
            )
        })?
    } else if allow_missing_leaf {
        let Some(leaf) = absolute.file_name() else {
            return Err(io_error(
                Some(path),
                "junction link path must include a directory name".to_string(),
            ));
        };
        let parent = absolute
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| {
                io_error(
                    Some(path),
                    "junction link path must include a parent directory".to_string(),
                )
            })?;
        let canonical_parent = if parent.exists() {
            fs::canonicalize(parent).map_err(|err| {
                io_error(
                    Some(path),
                    format!("failed to canonicalize parent path: {}", err),
                )
            })?
        } else {
            windows_normalize_separators(parent.to_path_buf())
        };
        canonical_parent.join(leaf)
    } else {
        return Err(io_error(
            Some(path),
            format!(
                "junction source path does not exist: {}",
                absolute.display()
            ),
        ));
    };

    Ok(windows_normalize_separators(normalized))
}

#[cfg(windows)]
fn windows_normalize_separators(path: PathBuf) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('/', "\\"))
}

#[cfg(not(windows))]
fn create_junction(source: &Path, link_path: &Path) -> Result<(), AppError> {
    create_dir_symlink(source, link_path)
}

#[cfg(windows)]
fn create_dir_symlink(source: &Path, link_path: &Path) -> Result<(), AppError> {
    std::os::windows::fs::symlink_dir(source, link_path).map_err(|err| {
        io_error(
            Some(link_path),
            format!("failed to create directory symlink: {}", err),
        )
    })
}

#[cfg(unix)]
fn create_dir_symlink(source: &Path, link_path: &Path) -> Result<(), AppError> {
    std::os::unix::fs::symlink(source, link_path).map_err(|err| {
        io_error(
            Some(link_path),
            format!("failed to create directory symlink: {}", err),
        )
    })
}

fn is_link(path: &Path) -> Result<bool, AppError> {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .map_err(|err| io_error(Some(path), format!("failed to inspect path: {}", err)))
}

fn same_target(actual_target: &Path, expected_target: &Path) -> bool {
    match (
        fs::canonicalize(actual_target),
        fs::canonicalize(expected_target),
    ) {
        (Ok(actual), Ok(expected)) => actual == expected,
        _ => actual_target == expected_target,
    }
}

fn io_error(path: Option<&Path>, message: String) -> AppError {
    AppError::Io {
        path: path.map(Path::to_path_buf),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_link_type_matches_current_os() {
        if cfg!(windows) {
            assert_eq!(default_link_type(), LinkType::Junction);
        } else {
            assert_eq!(default_link_type(), LinkType::Symlink);
        }
    }

    #[test]
    fn create_and_remove_symlink_or_junction_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let link = temp.path().join("linked");
        fs::create_dir(&source).expect("create source");
        fs::write(source.join("SKILL.md"), "content").expect("write source file");

        create_dir_link(&source, &link, default_link_type()).expect("create link");

        assert!(path_exists(&link));
        assert!(is_dir(&link));
        assert_eq!(
            fs::read_to_string(link.join("SKILL.md")).unwrap(),
            "content"
        );

        remove_recorded_link(&link, &source).expect("remove recorded link");

        assert!(!path_exists(&link));
        assert!(source.is_dir());
        assert!(source.join("SKILL.md").is_file());
    }

    #[test]
    fn link_target_returns_expected_source_for_link() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let link = temp.path().join("linked");
        fs::create_dir(&source).expect("create source");

        create_dir_link(&source, &link, default_link_type()).expect("create link");

        let target = link_target(&link)
            .expect("link target should resolve")
            .expect("path should be a link");
        assert!(same_target(&target, &source));
    }

    #[test]
    fn remove_recorded_link_refuses_unknown_real_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let real_dir = temp.path().join("real-dir");
        fs::create_dir(&source).expect("create source");
        fs::create_dir(&real_dir).expect("create real dir");
        fs::write(real_dir.join("keep.txt"), "keep").expect("write file");

        let error = remove_recorded_link(&real_dir, &source).expect_err("real dir should fail");

        assert!(matches!(error, AppError::Io { .. }));
        assert!(real_dir.is_dir());
        assert!(real_dir.join("keep.txt").is_file());
    }

    #[test]
    fn remove_recorded_link_refuses_unknown_regular_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let file = temp.path().join("regular-file");
        fs::create_dir(&source).expect("create source");
        fs::write(&file, "keep").expect("write file");

        let error = remove_recorded_link(&file, &source).expect_err("file should fail");

        assert!(matches!(error, AppError::Io { .. }));
        assert_eq!(fs::read_to_string(&file).unwrap(), "keep");
    }

    #[test]
    fn remove_recorded_link_fails_when_target_mismatch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let other_source = temp.path().join("other-source");
        let link = temp.path().join("linked");
        fs::create_dir(&source).expect("create source");
        fs::create_dir(&other_source).expect("create other source");

        create_dir_link(&source, &link, default_link_type()).expect("create link");

        let error = remove_recorded_link(&link, &other_source).expect_err("mismatch should fail");

        assert!(matches!(error, AppError::Io { .. }));
        assert!(path_exists(&link));
        assert!(source.is_dir());
    }

    #[test]
    fn delete_real_dir_removes_directory_tree() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path().join("source");
        let child = dir.join("child");
        fs::create_dir(&dir).expect("create dir");
        fs::create_dir(&child).expect("create child");
        fs::write(child.join("file.txt"), "content").expect("write file");

        delete_real_dir(&dir).expect("delete real dir");

        assert!(!path_exists(&dir));
    }

    #[test]
    fn create_junction_accepts_forward_slash_paths_on_windows() {
        if !cfg!(windows) {
            return;
        }

        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let skills_dir = temp.path().join(".cursor").join("skills");
        fs::create_dir_all(&source).expect("create source");
        fs::create_dir_all(&skills_dir).expect("create skills dir");
        fs::write(source.join("SKILL.md"), "content").expect("write source file");

        let mixed_link = PathBuf::from(format!(
            "{}/oxygen-tool-branch",
            skills_dir.to_string_lossy().replace('\\', "/")
        ));

        create_dir_link(&source, &mixed_link, LinkType::Junction).expect("create junction");

        assert!(path_exists(&mixed_link));
        assert_eq!(
            fs::read_to_string(mixed_link.join("SKILL.md")).unwrap(),
            "content"
        );
    }

    #[test]
    fn path_exists_and_is_dir_cover_basic_cases() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path().join("dir");
        let file = temp.path().join("file.txt");
        let missing = temp.path().join("missing");
        fs::create_dir(&dir).expect("create dir");
        fs::write(&file, "content").expect("write file");

        assert!(path_exists(&dir));
        assert!(is_dir(&dir));
        assert!(path_exists(&file));
        assert!(!is_dir(&file));
        assert!(!path_exists(&missing));
        assert!(!is_dir(&missing));
    }
}
