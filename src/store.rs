use std::path::{Path, PathBuf};
use std::io::Read;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

use crate::HarborError;

const HARBOR_DIR: &str = ".harbor";
const INDEX_FILE: &str = "index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub repo_name: String,
    pub commit: String,
    pub tag: String,
    pub description: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub stored_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Index {
    pub artifacts: Vec<Artifact>,
}

/// Get the harbor storage root (inside the repo)
fn harbor_root(repo: &Path) -> PathBuf {
    repo.join(HARBOR_DIR)
}

fn index_path(repo: &Path) -> PathBuf {
    harbor_root(repo).join(INDEX_FILE)
}

fn blob_path(repo: &Path, id: &str) -> PathBuf {
    harbor_root(repo).join("blobs").join(id)
}

/// Load the artifact index
pub fn load_index(repo: &Path) -> Result<Index, HarborError> {
    let path = index_path(repo);
    if !path.exists() {
        return Ok(Index::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let index: Index = serde_json::from_str(&content)?;
    Ok(index)
}

/// Save the artifact index
fn save_index(repo: &Path, index: &Index) -> Result<(), HarborError> {
    let path = index_path(repo);
    let content = serde_json::to_string_pretty(index)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Ensure harbor directories exist and .gitignore is set
fn ensure_dirs(repo: &Path) -> Result<(), HarborError> {
    let root = harbor_root(repo);
    std::fs::create_dir_all(root.join("blobs"))?;

    let gitignore = root.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, "*\n")?;
    }
    Ok(())
}

/// Get the current git commit hash
fn current_commit(repo: &Path) -> Result<String, HarborError> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()?;

    if !output.status.success() {
        return Err(HarborError::Validation(
            "Not a git repository or no commits yet".into(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get the repo name from the directory
fn repo_name(repo: &Path) -> String {
    repo.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Store an artifact from a file or stdin
pub fn store(
    repo: &Path,
    tag: &str,
    file: Option<&Path>,
    desc: Option<&str>,
) -> Result<Artifact, HarborError> {
    ensure_dirs(repo)?;

    let data = match file {
        Some(path) => std::fs::read(path)?,
        None => {
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf)?;
            if buf.is_empty() {
                return Err(HarborError::Validation(
                    "No input data. Provide --file or pipe to stdin.".into(),
                ));
            }
            buf
        }
    };

    let commit = current_commit(repo)?;
    let name = repo_name(repo);

    // Generate ID from content hash (first 12 chars)
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let full_hash = format!("{:x}", hasher.finalize());
    let id = format!("{}_{}", &commit[..8.min(commit.len())], &full_hash[..8]);

    let artifact = Artifact {
        id: id.clone(),
        repo_name: name,
        commit,
        tag: tag.to_string(),
        description: desc.unwrap_or("").to_string(),
        size_bytes: data.len() as u64,
        sha256: full_hash,
        stored_at: Utc::now(),
    };

    // Write blob
    let blob = blob_path(repo, &id);
    std::fs::write(&blob, &data)?;

    // Update index
    let mut index = load_index(repo)?;
    index.artifacts.push(artifact.clone());
    save_index(repo, &index)?;

    Ok(artifact)
}

/// List artifacts with optional filters
pub fn list(
    repo: &Path,
    commit_filter: Option<&str>,
    tag_filter: Option<&str>,
) -> Result<Vec<Artifact>, HarborError> {
    let index = load_index(repo)?;

    let filtered: Vec<Artifact> = index
        .artifacts
        .into_iter()
        .filter(|a| {
            if let Some(c) = commit_filter {
                if !a.commit.starts_with(c) {
                    return false;
                }
            }
            if let Some(t) = tag_filter {
                if a.tag != t {
                    return false;
                }
            }
            true
        })
        .collect();

    Ok(filtered)
}

/// Show artifact contents
pub fn show(repo: &Path, id: &str) -> Result<(Artifact, String), HarborError> {
    let index = load_index(repo)?;
    let artifact = index
        .artifacts
        .iter()
        .find(|a| a.id == id || a.id.starts_with(id))
        .ok_or_else(|| HarborError::Validation(format!("Artifact not found: {id}")))?
        .clone();

    let blob = blob_path(repo, &artifact.id);
    let content = std::fs::read_to_string(&blob)
        .unwrap_or_else(|_| "(binary data)".into());

    Ok((artifact, content))
}

/// Clean old artifacts, returns list of removed artifacts
pub fn clean(
    repo: &Path,
    older_than_days: Option<u64>,
    keep_per_tag: Option<usize>,
    dry_run: bool,
) -> Result<Vec<Artifact>, HarborError> {
    let mut index = load_index(repo)?;
    let now = Utc::now();
    let mut removed = Vec::new();

    // Filter by age
    if let Some(days) = older_than_days {
        let cutoff = now - chrono::Duration::days(days as i64);
        let (keep, remove): (Vec<_>, Vec<_>) = index
            .artifacts
            .into_iter()
            .partition(|a| a.stored_at >= cutoff);

        removed.extend(remove);
        index.artifacts = keep;
    }

    // Keep only N per tag
    if let Some(keep_n) = keep_per_tag {
        let mut by_tag: std::collections::HashMap<String, Vec<Artifact>> =
            std::collections::HashMap::new();

        for a in index.artifacts.drain(..) {
            by_tag.entry(a.tag.clone()).or_default().push(a);
        }

        for (_tag, mut arts) in by_tag {
            // Sort newest first
            arts.sort_by(|a, b| b.stored_at.cmp(&a.stored_at));

            for (i, a) in arts.into_iter().enumerate() {
                if i < keep_n {
                    index.artifacts.push(a);
                } else {
                    removed.push(a);
                }
            }
        }
    }

    if !dry_run {
        // Delete blob files
        for a in &removed {
            let blob = blob_path(repo, &a.id);
            let _ = std::fs::remove_file(&blob);
        }
        save_index(repo, &index)?;
    }

    Ok(removed)
}

/// Compute warehouse stats
pub fn stats(repo: &Path) -> Result<WarehouseStats, HarborError> {
    let index = load_index(repo)?;

    let total_size: u64 = index.artifacts.iter().map(|a| a.size_bytes).sum();
    let mut tags: Vec<String> = index.artifacts.iter().map(|a| a.tag.clone()).collect();
    tags.sort();
    tags.dedup();

    let mut commits: Vec<String> = index.artifacts.iter().map(|a| a.commit.clone()).collect();
    commits.sort();
    commits.dedup();

    Ok(WarehouseStats {
        artifact_count: index.artifacts.len(),
        total_size_bytes: total_size,
        unique_tags: tags.len(),
        unique_commits: commits.len(),
        tags,
    })
}

#[derive(Debug, Serialize)]
pub struct WarehouseStats {
    pub artifact_count: usize,
    pub total_size_bytes: u64,
    pub unique_tags: usize,
    pub unique_commits: usize,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) {
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn store_and_list() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();
        init_git_repo(repo);

        // Write a test file to store
        let test_file = repo.join("build.log");
        std::fs::write(&test_file, "BUILD OK\n").unwrap();

        let artifact = store(repo, "build-log", Some(&test_file), Some("test build")).unwrap();
        assert_eq!(artifact.tag, "build-log");
        assert_eq!(artifact.description, "test build");
        assert!(artifact.size_bytes > 0);

        let all = list(repo, None, None).unwrap();
        assert_eq!(all.len(), 1);

        let by_tag = list(repo, None, Some("build-log")).unwrap();
        assert_eq!(by_tag.len(), 1);

        let by_tag_miss = list(repo, None, Some("nope")).unwrap();
        assert_eq!(by_tag_miss.len(), 0);
    }

    #[test]
    fn show_artifact() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();
        init_git_repo(repo);

        let test_file = repo.join("output.txt");
        std::fs::write(&test_file, "hello harbor").unwrap();

        let artifact = store(repo, "output", Some(&test_file), None).unwrap();
        let (found, content) = show(repo, &artifact.id).unwrap();
        assert_eq!(found.id, artifact.id);
        assert_eq!(content, "hello harbor");
    }

    #[test]
    fn clean_by_keep() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();
        init_git_repo(repo);

        // Store 3 artifacts with same tag
        for i in 0..3 {
            let f = repo.join(format!("file{i}.txt"));
            std::fs::write(&f, format!("data {i}")).unwrap();
            store(repo, "log", Some(&f), None).unwrap();
        }

        let all = list(repo, None, None).unwrap();
        assert_eq!(all.len(), 3);

        let removed = clean(repo, None, Some(1), false).unwrap();
        assert_eq!(removed.len(), 2);

        let remaining = list(repo, None, None).unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn stats_counting() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();
        init_git_repo(repo);

        let f1 = repo.join("a.txt");
        std::fs::write(&f1, "aaa").unwrap();
        store(repo, "alpha", Some(&f1), None).unwrap();

        let f2 = repo.join("b.txt");
        std::fs::write(&f2, "bbb").unwrap();
        store(repo, "beta", Some(&f2), None).unwrap();

        let s = stats(repo).unwrap();
        assert_eq!(s.artifact_count, 2);
        assert_eq!(s.unique_tags, 2);
        assert_eq!(s.unique_commits, 1);
    }
}
