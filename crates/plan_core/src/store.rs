//! Atomic, crash-safe persistence for `plan.json`: temp-write + fsync + rename,
//! `.bak` rotation, and load-with-migration. See PRD §11 and F11.5.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::schema::{Plan, migrate};

/// How many `.bak` copies to retain per plan id.
const MAX_BACKUPS: usize = 10;

fn plan_path(plans_dir: &Path, id: &str) -> PathBuf {
    plans_dir.join(format!("{id}.plan.json"))
}

/// Load `<plans_dir>/<id>.plan.json`, migrating it to the current schema
/// version before typed deserialization. Returns an error (never panics) for a
/// missing or malformed file.
pub fn load(plans_dir: &Path, id: &str) -> Result<Plan> {
    let path = plan_path(plans_dir, id);
    let raw =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let value: Value =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    let migrated = migrate(value)?;
    serde_json::from_value(migrated).with_context(|| format!("deserializing {}", path.display()))
}

/// Enumerate plan ids in a directory (files named `<id>.plan.json`), sorted.
/// Ignores backups, temp files, and anything else.
pub fn list_plan_ids(plans_dir: &Path) -> Vec<String> {
    let mut ids: Vec<String> = match fs::read_dir(plans_dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .strip_suffix(".plan.json")
                    .map(String::from)
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    ids.sort();
    ids
}

/// Find the plan owned by `thread` (F1.0: one thread ↔ one plan). Returns the
/// first plan whose `thread` field matches, or `None`.
pub fn find_by_thread(plans_dir: &Path, thread: &str) -> Option<Plan> {
    list_plan_ids(plans_dir)
        .into_iter()
        .filter_map(|id| load(plans_dir, &id).ok())
        .find(|plan| plan.thread == thread)
}

/// Persist a plan atomically: back up any existing revision, write to a temp
/// file in the same directory, fsync it, then rename over the target. The
/// rename is atomic within a filesystem, so a crash never leaves a torn file.
pub fn save(plans_dir: &Path, plan: &Plan) -> Result<()> {
    fs::create_dir_all(plans_dir)
        .with_context(|| format!("creating {}", plans_dir.display()))?;
    let target = plan_path(plans_dir, &plan.id);
    if target.exists() {
        backup(plans_dir, &plan.id)?;
    }

    let tmp = plans_dir.join(format!("{}.plan.json.tmp.{}", plan.id, std::process::id()));
    let bytes = serde_json::to_vec_pretty(plan)?;
    {
        let mut file =
            fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        file.write_all(&bytes)?;
        file.flush()?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &target)
        .with_context(|| format!("renaming into {}", target.display()))?;
    Ok(())
}

/// Copy the current on-disk plan into `.bak/<id>.rev<N>.plan.json` (keyed by the
/// revision being replaced) before it is overwritten, then prune to the newest
/// [`MAX_BACKUPS`].
fn backup(plans_dir: &Path, id: &str) -> Result<()> {
    let target = plan_path(plans_dir, id);
    let existing =
        fs::read_to_string(&target).with_context(|| format!("reading {}", target.display()))?;
    let rev = serde_json::from_str::<Value>(&existing)
        .ok()
        .and_then(|value| value.get("rev").and_then(Value::as_u64))
        .unwrap_or(0);

    let bak_dir = plans_dir.join(".bak");
    fs::create_dir_all(&bak_dir)
        .with_context(|| format!("creating {}", bak_dir.display()))?;
    let bak = bak_dir.join(format!("{id}.rev{rev}.plan.json"));
    fs::write(&bak, existing).with_context(|| format!("writing {}", bak.display()))?;

    prune_backups(&bak_dir, id)?;
    Ok(())
}

/// Keep only the [`MAX_BACKUPS`] highest-revision backups for `id`.
fn prune_backups(bak_dir: &Path, id: &str) -> Result<()> {
    let prefix = format!("{id}.rev");
    let mut backups: Vec<(u64, PathBuf)> = fs::read_dir(bak_dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rev = name
                .strip_prefix(&prefix)?
                .strip_suffix(".plan.json")?
                .parse::<u64>()
                .ok()?;
            Some((rev, entry.path()))
        })
        .collect();
    if backups.len() <= MAX_BACKUPS {
        return Ok(());
    }
    backups.sort_by_key(|(rev, _)| *rev);
    let remove = backups.len() - MAX_BACKUPS;
    for (_, path) in backups.into_iter().take(remove) {
        fs::remove_file(&path).with_context(|| format!("pruning {}", path.display()))?;
    }
    Ok(())
}
