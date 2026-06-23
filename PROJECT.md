# PROJECT.md — harbor

**What:** Build artifact warehouse — stores build outputs by repo+commit+tag for agent recall.

**Status:** MVP complete, published to github.com/MPfeifer33/harbor

## Architecture
- `src/cli.rs` — Clap 4 CLI: `store` (--tag, --file, --desc), `list` (--commit, --tag), `show`, `clean` (--older-than, --keep, --dry-run), `stats`
- `src/store.rs` — Artifact storage in .harbor/blobs/ with JSON index. SHA256 content hashing, git commit association, age/count-based cleanup. 4 unit tests.
- `src/report.rs` — Stored, list, show, clean, stats output (text + JSON)
- `src/main.rs` — Standard error handling

## Usage
```bash
# Store a build log
harbor store --tag build-log --file build.log --desc "Release build"

# Store from pipeline output
cargo build 2>&1 | harbor store --tag build-output

# List all artifacts
harbor list

# Filter by commit or tag
harbor list --commit abc123 --tag build-log

# Show artifact contents
harbor show <artifact-id>

# Clean old artifacts
harbor clean --older-than 30
harbor clean --keep 5 --dry-run

# Warehouse stats
harbor stats
```

## Design Decisions
- Content-addressed storage (SHA256 blob IDs)
- .harbor/ directory is gitignored by default
- Artifacts tied to git commits for traceability
- Prefix-match on artifact IDs for convenience (don't need full ID)
- Clean supports both age-based and count-based retention

## Last Updated
June 22, 2026 — Initial MVP
