use crate::store::{Artifact, WarehouseStats};
use crate::HarborError;

pub fn print_stored(artifact: &Artifact, is_json: bool) -> Result<(), HarborError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "stored": artifact,
        }))?);
    } else {
        println!("Stored artifact:");
        println!("  ID:     {}", artifact.id);
        println!("  Tag:    {}", artifact.tag);
        println!("  Commit: {}", &artifact.commit[..12.min(artifact.commit.len())]);
        println!("  Size:   {} bytes", artifact.size_bytes);
        println!("  SHA256: {}…", &artifact.sha256[..16]);
        if !artifact.description.is_empty() {
            println!("  Desc:   {}", artifact.description);
        }
    }
    Ok(())
}

pub fn print_list(artifacts: &[Artifact], is_json: bool) -> Result<(), HarborError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "artifacts": artifacts,
            "count": artifacts.len(),
        }))?);
    } else {
        if artifacts.is_empty() {
            println!("No artifacts found.");
        } else {
            println!("{} artifact(s):", artifacts.len());
            println!();
            for a in artifacts {
                let commit_short = &a.commit[..8.min(a.commit.len())];
                let desc = if a.description.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", a.description)
                };
                println!("  {} [{}] @{} ({} bytes){}", a.id, a.tag, commit_short, a.size_bytes, desc);
            }
        }
    }
    Ok(())
}

pub fn print_show(artifact: &Artifact, content: &str, is_json: bool) -> Result<(), HarborError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "artifact": artifact,
            "content": content,
        }))?);
    } else {
        println!("── {} [{}] ──", artifact.id, artifact.tag);
        println!();
        println!("{content}");
    }
    Ok(())
}

pub fn print_clean(removed: &[Artifact], dry_run: bool, is_json: bool) -> Result<(), HarborError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "dry_run": dry_run,
            "removed": removed,
            "count": removed.len(),
        }))?);
    } else {
        let verb = if dry_run { "Would remove" } else { "Removed" };
        if removed.is_empty() {
            println!("Nothing to clean.");
        } else {
            println!("{verb} {} artifact(s):", removed.len());
            for a in removed {
                println!("  - {} [{}] ({} bytes)", a.id, a.tag, a.size_bytes);
            }
        }
    }
    Ok(())
}

pub fn print_stats(stats: &WarehouseStats, is_json: bool) -> Result<(), HarborError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "stats": stats,
        }))?);
    } else {
        println!("Harbor warehouse:");
        println!("  Artifacts: {}", stats.artifact_count);
        println!("  Total size: {} bytes", stats.total_size_bytes);
        println!("  Unique tags: {}", stats.unique_tags);
        println!("  Unique commits: {}", stats.unique_commits);
        if !stats.tags.is_empty() {
            println!("  Tags: {}", stats.tags.join(", "));
        }
    }
    Ok(())
}
