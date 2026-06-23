mod cli;
mod report;
mod store;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = run(&cli);
    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            if cli.is_json() {
                let err_json = serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": e.error_code(),
                        "message": e.to_string(),
                    }
                });
                eprintln!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(code);
        }
    }
}

fn run(cli: &Cli) -> Result<(), HarborError> {
    let repo = cli.resolve_repo()?;

    match &cli.command {
        Command::Store { tag, file, desc } => {
            let artifact = store::store(
                &repo,
                tag,
                file.as_deref(),
                desc.as_deref(),
            )?;
            report::print_stored(&artifact, cli.is_json())
        }
        Command::List { commit, tag } => {
            let artifacts = store::list(
                &repo,
                commit.as_deref(),
                tag.as_deref(),
            )?;
            report::print_list(&artifacts, cli.is_json())
        }
        Command::Show { id } => {
            let (artifact, content) = store::show(&repo, id)?;
            report::print_show(&artifact, &content, cli.is_json())
        }
        Command::Clean { older_than, keep, dry_run } => {
            let removed = store::clean(&repo, *older_than, *keep, *dry_run)?;
            report::print_clean(&removed, *dry_run, cli.is_json())
        }
        Command::Stats => {
            let s = store::stats(&repo)?;
            report::print_stats(&s, cli.is_json())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HarborError {
    #[error("{0}")]
    Validation(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl HarborError {
    pub fn exit_code(&self) -> i32 {
        match self {
            HarborError::Validation(_) => 1,
            HarborError::Io(_) => 2,
            HarborError::Json(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            HarborError::Validation(_) => "validation_error",
            HarborError::Io(_) => "io_error",
            HarborError::Json(_) => "json_error",
        }
    }
}
