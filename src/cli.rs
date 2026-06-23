use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "harbor",
    about = "Local build artifact warehouse — stores outputs by repo+commit for agent recall"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Repository root (default: current directory)
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Store a build artifact (file or stdin) tagged by repo+commit
    Store {
        /// Tag name for this artifact (e.g. "build-log", "test-output", "binary")
        #[arg(long)]
        tag: String,

        /// File to store (reads from stdin if omitted)
        #[arg(long)]
        file: Option<PathBuf>,

        /// Optional description
        #[arg(long)]
        desc: Option<String>,
    },

    /// List stored artifacts for a repo, optionally filtered by commit or tag
    List {
        /// Filter by commit hash (prefix match)
        #[arg(long)]
        commit: Option<String>,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },

    /// Show contents of a stored artifact
    Show {
        /// Artifact ID (from `harbor list`)
        id: String,
    },

    /// Remove old artifacts by age or count
    Clean {
        /// Remove artifacts older than N days
        #[arg(long)]
        older_than: Option<u64>,

        /// Keep only the latest N artifacts per tag
        #[arg(long)]
        keep: Option<usize>,

        /// Dry run — show what would be removed
        #[arg(long)]
        dry_run: bool,
    },

    /// Show warehouse statistics
    Stats,
}

impl Cli {
    pub fn resolve_repo(&self) -> Result<PathBuf, crate::HarborError> {
        match &self.repo {
            Some(p) => Ok(p.clone()),
            None => std::env::current_dir().map_err(|e| e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn rejects_unknown_output_format() {
        let result = Cli::try_parse_from(["harbor", "--format", "yaml", "stats"]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_json_output_format() {
        let cli = Cli::try_parse_from(["harbor", "--format", "json", "stats"]).unwrap();
        assert_eq!(cli.format, OutputFormat::Json);
    }
}
