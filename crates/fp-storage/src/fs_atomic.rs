use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const STAGING_DIR_NAME: &str = ".fp_staging";
pub const TRASH_DIR_NAME: &str = ".fp_trash";
pub const JOURNAL_DIR_NAME: &str = ".fp_journal";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteJournalEntry {
    pub project_id: String,
    pub project_slug: String,
    pub original_dir: String,
    pub trash_dir: String,
    pub created_at: String,
}

/// Checks whether a filesystem path is a symlink or Windows NTFS junction/reparse point.
pub fn is_reparse_or_symlink(path: &Path) -> bool {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return true;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
            if (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
                return true;
            }
        }
    }
    false
}

/// Validates a relative path inside a project directory:
/// - Rejects empty, absolute, drive prefix, root dir, `..`, backslashes escaping, and null bytes.
/// - Rejects any symlink or Windows NTFS junction along existing components.
pub fn validate_safe_rel_path(project_dir: &Path, rel_path: &str) -> Result<(String, PathBuf)> {
    let trimmed = rel_path.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        bail!("PROJECT_ISOLATION_VIOLATION: Invalid empty or null relative path");
    }
    let normalized = trimmed.replace('\\', "/");
    if normalized.starts_with('/') || normalized.contains(':') {
        bail!(
            "PROJECT_ISOLATION_VIOLATION: Absolute or drive-prefixed path rejected: '{}'",
            rel_path
        );
    }

    let p = Path::new(&normalized);
    let mut built = project_dir.to_path_buf();
    for comp in p.components() {
        match comp {
            Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s == "." || s == ".." {
                    bail!(
                        "PROJECT_ISOLATION_VIOLATION: Relative traversal rejected in '{}'",
                        rel_path
                    );
                }
                built.push(seg);
                if built.exists() && is_reparse_or_symlink(&built) {
                    bail!(
                        "PROJECT_ISOLATION_VIOLATION: Symlink/junction rejected at '{}'",
                        built.display()
                    );
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Unsafe path component in '{}'",
                    rel_path
                );
            }
        }
    }

    if project_dir.exists() {
        let canon_root = std::fs::canonicalize(project_dir).with_context(|| {
            format!(
                "Failed to canonicalize project dir: {}",
                project_dir.display()
            )
        })?;
        if built.exists() {
            let canon_target = std::fs::canonicalize(&built)?;
            if !canon_target.starts_with(&canon_root) {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Target '{}' escapes project root '{}'",
                    canon_target.display(),
                    canon_root.display()
                );
            }
        } else if let Some(parent) = built.parent() {
            if parent.exists() {
                let canon_parent = std::fs::canonicalize(parent)?;
                if !canon_parent.starts_with(&canon_root) {
                    bail!(
                        "PROJECT_ISOLATION_VIOLATION: Parent '{}' escapes project root '{}'",
                        canon_parent.display(),
                        canon_root.display()
                    );
                }
            }
        }
    }

    Ok((normalized, built))
}

/// Validates that `target_dir` is safely contained inside `projects_root` (no `..`, no symlink/junction escape).
pub fn ensure_contained_project_dir(projects_root: &Path, target_dir: &Path) -> Result<PathBuf> {
    if target_dir
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        bail!(
            "PROJECT_ISOLATION_VIOLATION: Parent traversal '..' rejected in '{}'",
            target_dir.display()
        );
    }

    std::fs::create_dir_all(projects_root)?;
    if is_reparse_or_symlink(projects_root) {
        bail!(
            "PROJECT_ISOLATION_VIOLATION: projects_root '{}' cannot be a symlink/junction",
            projects_root.display()
        );
    }
    let canon_root = std::fs::canonicalize(projects_root)?;

    // Check any existing prefix from root down to target_dir
    if target_dir.exists() {
        if is_reparse_or_symlink(target_dir) {
            bail!(
                "PROJECT_ISOLATION_VIOLATION: Project directory '{}' is a symlink/junction",
                target_dir.display()
            );
        }
        let canon_target = std::fs::canonicalize(target_dir)?;
        if !canon_target.starts_with(&canon_root) || canon_target == canon_root {
            bail!(
                "PROJECT_ISOLATION_VIOLATION: Directory '{}' is not strictly inside projects_root '{}'",
                canon_target.display(),
                canon_root.display()
            );
        }
        return Ok(canon_target);
    }

    // Target doesn't exist yet: walk up to first existing ancestor and verify containment + non-symlink
    let mut probe = target_dir.parent();
    while let Some(ancestor) = probe {
        if ancestor.exists() {
            if is_reparse_or_symlink(ancestor) {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Ancestor '{}' is a symlink/junction",
                    ancestor.display()
                );
            }
            let canon_anc = std::fs::canonicalize(ancestor)?;
            if !canon_anc.starts_with(&canon_root) {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Ancestor '{}' escapes projects_root '{}'",
                    canon_anc.display(),
                    canon_root.display()
                );
            }
            break;
        }
        probe = ancestor.parent();
    }
    Ok(target_dir.to_path_buf())
}

/// Atomically writes bytes to `target` using a temp file in the same directory + `sync_all` + `rename`.
pub fn atomic_write_bytes(target: &Path, bytes: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .context("atomic_write_bytes target must have a parent directory")?;
    std::fs::create_dir_all(parent)?;

    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("artifact");
    let tmp_path = parent.join(format!(".{}.tmp.{}", file_name, uuid::Uuid::new_v4()));

    {
        let mut f = std::fs::File::create(&tmp_path)
            .with_context(|| format!("Failed creating temp file {}", tmp_path.display()))?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }

    if let Err(e) = std::fs::rename(&tmp_path, target) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).with_context(|| {
            format!(
                "Atomic rename failed from {} to {}",
                tmp_path.display(),
                target.display()
            )
        });
    }
    Ok(())
}

/// Safely copies regular files and subdirectories from `src` to `dst`, rejecting symlinks/junctions.
pub fn copy_dir_files_safe(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if is_reparse_or_symlink(src) {
        bail!(
            "PROJECT_ISOLATION_VIOLATION: Refusing to copy from symlink/junction '{}'",
            src.display()
        );
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let p = entry.path();
        if is_reparse_or_symlink(&p) {
            bail!(
                "PROJECT_ISOLATION_VIOLATION: Symlink/junction entry rejected during copy: '{}'",
                p.display()
            );
        }
        let target = dst.join(entry.file_name());
        if p.is_dir() {
            copy_dir_files_safe(&p, &target)?;
        } else if p.is_file() {
            let bytes = std::fs::read(&p)?;
            atomic_write_bytes(&target, &bytes)?;
        }
    }
    Ok(())
}

/// Cleans up orphan `.fp_staging/*` and committed `.fp_trash/*` / `.fp_journal/*` directories
/// left behind by a prior mid-operation crash.
/// - Any `.fp_staging/*` folder is uncommitted by definition (since `tx.commit()` only runs AFTER rename out of staging) -> safe to purge!
/// - Any `.fp_journal/delete_<id>.json`: if `project_id` is NOT in `active_project_ids`, DB delete committed -> purge `trash_dir` & journal;
///   if `project_id` IS still in `active_project_ids` and `original_dir` doesn't exist -> restore `trash_dir` back to `original_dir`!
///   Journal and trash entries are preserved if recovery fails so data is never silently lost.
pub fn reconcile_project_storage_orphans(
    projects_root: &Path,
    active_project_ids: &[String],
) -> Result<usize> {
    if !projects_root.exists() {
        return Ok(0);
    }
    let mut cleaned = 0usize;
    let canon_root = std::fs::canonicalize(projects_root)?;
    let trash_root = projects_root.join(TRASH_DIR_NAME);

    let staging_root = projects_root.join(STAGING_DIR_NAME);
    if staging_root.exists() {
        for entry in std::fs::read_dir(&staging_root)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                if std::fs::remove_dir_all(&p).is_ok() {
                    cleaned += 1;
                }
            } else if std::fs::remove_file(&p).is_ok() {
                cleaned += 1;
            }
        }
    }

    let mut protected_trash_paths: std::collections::HashSet<PathBuf> =
        std::collections::HashSet::new();

    let journal_root = projects_root.join(JOURNAL_DIR_NAME);
    if journal_root.exists() {
        for entry in std::fs::read_dir(&journal_root)? {
            let entry = entry?;
            let jpath = entry.path();
            if !jpath.is_file() {
                continue;
            }
            if let Ok(raw) = std::fs::read(&jpath) {
                if let Ok(item) = serde_json::from_slice::<DeleteJournalEntry>(&raw) {
                    let still_in_db = active_project_ids.iter().any(|id| id == &item.project_id);
                    let trash_p = PathBuf::from(&item.trash_dir);
                    let orig_p = PathBuf::from(&item.original_dir);

                    // Ensure journal paths stay strictly inside projects_root
                    let trash_inside = trash_p
                        .parent()
                        .and_then(|p| std::fs::canonicalize(p).ok())
                        .map(|cp| cp.starts_with(&canon_root))
                        .unwrap_or(false);
                    let orig_inside = orig_p
                        .parent()
                        .and_then(|p| std::fs::canonicalize(p).ok())
                        .map(|cp| cp.starts_with(&canon_root))
                        .unwrap_or(false);

                    let mut journal_resolved = false;
                    if trash_inside && orig_inside {
                        if still_in_db {
                            if trash_p.exists() && !orig_p.exists() {
                                if std::fs::rename(&trash_p, &orig_p).is_ok() {
                                    cleaned += 1;
                                    journal_resolved = true;
                                } else {
                                    protected_trash_paths.insert(
                                        std::fs::canonicalize(&trash_p).unwrap_or_else(|_| trash_p.clone()),
                                    );
                                }
                            } else if !trash_p.exists() {
                                journal_resolved = true;
                            } else {
                                protected_trash_paths.insert(
                                    std::fs::canonicalize(&trash_p).unwrap_or_else(|_| trash_p.clone()),
                                );
                            }
                        } else if trash_p.exists() {
                            if std::fs::remove_dir_all(&trash_p).is_ok() {
                                cleaned += 1;
                                journal_resolved = true;
                            } else {
                                protected_trash_paths.insert(
                                    std::fs::canonicalize(&trash_p).unwrap_or_else(|_| trash_p.clone()),
                                );
                            }
                        } else {
                            journal_resolved = true;
                        }
                    }

                    if journal_resolved {
                        let _ = std::fs::remove_file(&jpath);
                    }
                }
            }
        }
    }

    // Any remaining unreferenced dirs in .fp_trash (not protected by an unresolved journal) can be purged
    if trash_root.exists() {
        for entry in std::fs::read_dir(&trash_root)? {
            let entry = entry?;
            let p = entry.path();
            let p_cmp = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
            if protected_trash_paths.contains(&p_cmp) || protected_trash_paths.contains(&p) {
                continue;
            }
            if p.is_dir() {
                if std::fs::remove_dir_all(&p).is_ok() {
                    cleaned += 1;
                }
            } else if std::fs::remove_file(&p).is_ok() {
                cleaned += 1;
            }
        }
    }

    Ok(cleaned)
}
