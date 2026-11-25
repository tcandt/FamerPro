use crate::models::ReconcileReport;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJournalEntry {
    pub op: String, // "create" | "clone"
    pub project_id: String,
    pub project_slug: String,
    pub staging_dir: String,
    pub target_dir: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBackupEntry {
    pub live_path: String,
    pub existed_before: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MutationCommitWitness {
    ProjectUpdatedAt {
        project_id: String,
        expected_updated_at: String,
    },
    WorkflowVersionExists {
        project_id: String,
        workflow_id: String,
        version_id: String,
        expected_sha256: String,
    },
    VisionTemplateExists {
        project_id: String,
        rel_path: String,
        expected_sha256: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationJournalEntry {
    pub op: String, // "mutation"
    pub journal_id: String,
    pub project_id: String,
    pub backup_dir: String,
    pub files: Vec<FileBackupEntry>,
    pub commit_witness: MutationCommitWitness,
    pub created_at: String,
}

/// Witness interface checked during startup filesystem reconciliation against committed SQLite state.
/// SQLite transactions and NTFS directory/file operations reside in separate durability domains;
/// crash consistency is achieved by intent-journaling pre-images + verifying this SQLite commit witness.
pub trait CommitWitness {
    fn has_project(&self, project_id: &str) -> Result<bool>;
    fn active_project_dirs(&self) -> Result<Vec<PathBuf>>;
    fn matches_mutation_witness(&self, witness: &MutationCommitWitness) -> Result<bool>;
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

/// Atomically replaces `target` via temp file in same directory + `sync_all` + `rename`.
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

fn path_strictly_inside(candidate: &Path, base_canon: &Path) -> bool {
    if is_reparse_or_symlink(candidate) {
        return false;
    }
    if candidate.exists() {
        return std::fs::canonicalize(candidate)
            .map(|c| c.starts_with(base_canon) && c != base_canon)
            .unwrap_or(false);
    }
    candidate
        .parent()
        .and_then(|p| {
            if is_reparse_or_symlink(p) {
                return None;
            }
            std::fs::canonicalize(p).ok()
        })
        .map(|cp| cp.starts_with(base_canon))
        .unwrap_or(false)
}

fn is_atomic_tmp_file_name(name: &str) -> bool {
    name.starts_with('.') && name.contains(".tmp.")
}

fn purge_tmp_files_in_dir(dir: &Path, report: &mut ReconcileReport) {
    if !dir.exists() || is_reparse_or_symlink(dir) {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if is_atomic_tmp_file_name(name) && std::fs::remove_file(&p).is_ok() {
                        report.tmp_files_purged += 1;
                        report.total_resolved += 1;
                    }
                }
            } else if p.is_dir() && !is_reparse_or_symlink(&p) {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if matches!(name, "workflows" | "templates" | "branding") {
                        purge_tmp_files_in_dir(&p, report);
                    }
                }
            }
        }
    }
}

/// Reconciles `.fp_journal/*`, `.fp_staging/*`, `.fp_trash/*`, and orphan `.tmp.*` files against committed SQLite state.
/// Order matters:
/// 1. Process `.fp_journal/*` entries FIRST so pre-image backups in `.fp_staging/bak_*` are intact before staging sweep.
/// 2. Sweep remaining unprotected `.fp_staging/*` & `.fp_trash/*` entries.
/// 3. Sweep stray `.*.tmp.*` files inside active project directories.
pub fn reconcile_project_storage_with_witness(
    projects_root: &Path,
    witness: &dyn CommitWitness,
) -> Result<ReconcileReport> {
    let mut report = ReconcileReport::default();
    if !projects_root.exists() {
        return Ok(report);
    }
    let canon_root = std::fs::canonicalize(projects_root)?;
    let staging_root = projects_root.join(STAGING_DIR_NAME);
    let trash_root = projects_root.join(TRASH_DIR_NAME);
    let journal_root = projects_root.join(JOURNAL_DIR_NAME);

    let active_dirs: HashSet<PathBuf> = witness
        .active_project_dirs()?
        .into_iter()
        .map(|d| std::fs::canonicalize(&d).unwrap_or(d))
        .collect();

    let mut protected_staging_paths: HashSet<PathBuf> = HashSet::new();
    let mut protected_trash_paths: HashSet<PathBuf> = HashSet::new();

    // 1. Reconcile journals first
    if journal_root.exists() {
        let canon_staging = std::fs::canonicalize(&staging_root).ok();
        let canon_trash = std::fs::canonicalize(&trash_root).ok();

        for entry in std::fs::read_dir(&journal_root)? {
            let entry = entry?;
            let jpath = entry.path();
            if !jpath.is_file() {
                continue;
            }
            let Ok(raw) = std::fs::read(&jpath) else {
                continue;
            };
            let Ok(val) = serde_json::from_slice::<serde_json::Value>(&raw) else {
                continue;
            };
            let op_str = val.get("op").and_then(|v| v.as_str()).unwrap_or("");

            if op_str == "create" || op_str == "clone" {
                if let Ok(item) = serde_json::from_slice::<CreateJournalEntry>(&raw) {
                    let committed = witness.has_project(&item.project_id)?;
                    let stg_p = PathBuf::from(&item.staging_dir);
                    let tgt_p = PathBuf::from(&item.target_dir);
                    let tgt_canon = std::fs::canonicalize(&tgt_p).unwrap_or_else(|_| tgt_p.clone());

                    let stg_ok = canon_staging
                        .as_ref()
                        .map(|cs| path_strictly_inside(&stg_p, cs))
                        .unwrap_or(false);
                    let tgt_ok = path_strictly_inside(&tgt_p, &canon_root)
                        && !tgt_p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| {
                                n == STAGING_DIR_NAME
                                    || n == TRASH_DIR_NAME
                                    || n == JOURNAL_DIR_NAME
                            })
                            .unwrap_or(false);

                    let mut resolved = false;
                    if committed {
                        if stg_ok && stg_p.exists() {
                            let _ = std::fs::remove_dir_all(&stg_p);
                        }
                        report.creates_finalized += 1;
                        report.total_resolved += 1;
                        report.actions.push(format!(
                            "create_finalized:{}:kept_committed_dir",
                            item.project_id
                        ));
                        resolved = true;
                    } else {
                        if stg_ok && stg_p.exists() && std::fs::remove_dir_all(&stg_p).is_ok() {
                            report.staging_purged += 1;
                            report.total_resolved += 1;
                        }
                        if tgt_ok && tgt_p.exists() && !active_dirs.contains(&tgt_canon) {
                            let mf_path = tgt_p.join("project.json");
                            let id_matches_or_incomplete = if mf_path.exists() {
                                std::fs::read_to_string(&mf_path)
                                    .ok()
                                    .and_then(|s| {
                                        serde_json::from_str::<serde_json::Value>(&s).ok()
                                    })
                                    .and_then(|m| {
                                        m.get("projectId")
                                            .and_then(|x| x.as_str())
                                            .map(|pid| pid == item.project_id)
                                    })
                                    .unwrap_or(true)
                            } else {
                                true
                            };
                            if id_matches_or_incomplete && std::fs::remove_dir_all(&tgt_p).is_ok() {
                                report.uncommitted_dirs_reverted += 1;
                                report.total_resolved += 1;
                                report.actions.push(format!(
                                    "create_reverted:{}:removed_uncommitted_dir",
                                    item.project_id
                                ));
                                resolved = true;
                            }
                        } else if !tgt_p.exists() {
                            report
                                .actions
                                .push(format!("create_aborted_in_staging:{}", item.project_id));
                            resolved = true;
                        }
                    }

                    if resolved {
                        let _ = std::fs::remove_file(&jpath);
                    }
                    continue;
                }
            }

            if op_str == "mutation" {
                if let Ok(item) = serde_json::from_slice::<MutationJournalEntry>(&raw) {
                    let committed = witness.matches_mutation_witness(&item.commit_witness)?;
                    let bak_p = PathBuf::from(&item.backup_dir);
                    let bak_ok = canon_staging
                        .as_ref()
                        .map(|cs| path_strictly_inside(&bak_p, cs))
                        .unwrap_or(false);

                    let mut resolved = true;
                    if committed {
                        report.mutations_finalized += 1;
                        report.total_resolved += 1;
                        report.actions.push(format!(
                            "mutation_finalized:{}:kept_committed_files",
                            item.journal_id
                        ));
                    } else {
                        for f in &item.files {
                            let live_p = PathBuf::from(&f.live_path);
                            // Ensure target file is strictly inside a project subfolder of projects_root
                            let live_ok = live_p
                                .parent()
                                .filter(|par| !is_reparse_or_symlink(par))
                                .and_then(|par| std::fs::canonicalize(par).ok())
                                .map(|cp| cp.starts_with(&canon_root) && cp != canon_root)
                                .unwrap_or(false);
                            if !live_ok {
                                resolved = false;
                                continue;
                            }

                            if f.existed_before {
                                if let Some(ref bpath_str) = f.backup_path {
                                    let bpath = PathBuf::from(bpath_str);
                                    let b_ok = canon_staging
                                        .as_ref()
                                        .map(|cs| path_strictly_inside(&bpath, cs))
                                        .unwrap_or(false);
                                    if b_ok && bpath.exists() {
                                        if let Ok(bytes) = std::fs::read(&bpath) {
                                            if atomic_write_bytes(&live_p, &bytes).is_err() {
                                                resolved = false;
                                            }
                                        } else {
                                            resolved = false;
                                        }
                                    } else {
                                        resolved = false;
                                    }
                                } else {
                                    resolved = false;
                                }
                            } else if live_p.exists() && std::fs::remove_file(&live_p).is_err() {
                                resolved = false;
                            }

                            if let Some(par) = live_p.parent() {
                                purge_tmp_files_in_dir(par, &mut report);
                            }
                        }

                        if resolved {
                            report.mutations_rolled_back += 1;
                            report.total_resolved += 1;
                            report.actions.push(format!(
                                "mutation_rolled_back:{}:restored_pre_image",
                                item.journal_id
                            ));
                        } else if bak_ok && bak_p.exists() {
                            protected_staging_paths
                                .insert(std::fs::canonicalize(&bak_p).unwrap_or(bak_p.clone()));
                        }
                    }

                    if resolved {
                        if bak_ok && bak_p.exists() {
                            let _ = std::fs::remove_dir_all(&bak_p);
                        }
                        let _ = std::fs::remove_file(&jpath);
                    }
                    continue;
                }
            }

            // Legacy / explicit DeleteJournalEntry
            if let Ok(item) = serde_json::from_slice::<DeleteJournalEntry>(&raw) {
                let still_in_db = witness.has_project(&item.project_id)?;
                let trash_p = PathBuf::from(&item.trash_dir);
                let orig_p = PathBuf::from(&item.original_dir);

                let trash_inside = canon_trash
                    .as_ref()
                    .map(|ct| path_strictly_inside(&trash_p, ct))
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
                                report.deletes_restored += 1;
                                report.total_resolved += 1;
                                report.actions.push(format!(
                                    "delete_restored:{}:moved_back_from_trash",
                                    item.project_id
                                ));
                                journal_resolved = true;
                            } else {
                                protected_trash_paths.insert(
                                    std::fs::canonicalize(&trash_p)
                                        .unwrap_or_else(|_| trash_p.clone()),
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
                            report.deletes_finalized += 1;
                            report.total_resolved += 1;
                            report.actions.push(format!(
                                "delete_finalized:{}:purged_committed_trash",
                                item.project_id
                            ));
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

    // 2. Sweep remaining unprotected .fp_staging/* entries
    if staging_root.exists() {
        for entry in std::fs::read_dir(&staging_root)? {
            let entry = entry?;
            let p = entry.path();
            let p_cmp = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
            if protected_staging_paths.contains(&p_cmp) || protected_staging_paths.contains(&p) {
                continue;
            }
            if p.is_dir() {
                if std::fs::remove_dir_all(&p).is_ok() {
                    report.staging_purged += 1;
                    report.total_resolved += 1;
                }
            } else if std::fs::remove_file(&p).is_ok() {
                report.staging_purged += 1;
                report.total_resolved += 1;
            }
        }
    }

    // 3. Sweep remaining unprotected .fp_trash/* entries
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
                    report.trash_purged += 1;
                    report.total_resolved += 1;
                }
            } else if std::fs::remove_file(&p).is_ok() {
                report.trash_purged += 1;
                report.total_resolved += 1;
            }
        }
    }

    // 4. Sweep any stray `.*.tmp.*` inside active project dirs
    for d in &active_dirs {
        purge_tmp_files_in_dir(d, &mut report);
    }

    Ok(report)
}

struct IdListWitness<'a>(&'a [String]);
impl CommitWitness for IdListWitness<'_> {
    fn has_project(&self, project_id: &str) -> Result<bool> {
        Ok(self.0.iter().any(|id| id == project_id))
    }
    fn active_project_dirs(&self) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }
    fn matches_mutation_witness(&self, _witness: &MutationCommitWitness) -> Result<bool> {
        Ok(false)
    }
}

pub fn reconcile_project_storage_orphans(
    projects_root: &Path,
    active_project_ids: &[String],
) -> Result<usize> {
    let rep =
        reconcile_project_storage_with_witness(projects_root, &IdListWitness(active_project_ids))?;
    Ok(rep.total_resolved)
}
