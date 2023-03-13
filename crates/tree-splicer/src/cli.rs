use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use tracing::{error, warn};
use tracing_subscriber::fmt::format::FmtSpan;
use tree_sitter::Tree;

use crate::splice;
use crate::splice::Config;

mod formatter;

#[derive(clap::ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum OnParseError {
    Ignore,
    Warn,
    Error,
}

impl std::fmt::Display for OnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OnParseError::Ignore => write!(f, "ignore"),
            OnParseError::Warn => write!(f, "warn"),
            OnParseError::Error => write!(f, "error"),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for OnParseError {
    fn default() -> Self {
        OnParseError::Ignore
    }
}

fn handle_parse_errors(path: &str, tree: &Tree, on_parse_error: &OnParseError) {
    let node = tree.root_node();
    match on_parse_error {
        OnParseError::Ignore => (),
        OnParseError::Warn if !node.has_error() => (),
        OnParseError::Error if !node.has_error() => (),
        OnParseError::Warn => {
            warn!(path, "Parse error in {}", path);
        }
        OnParseError::Error => {
            error!(path, "Parse error in {}", path);
            process::exit(1);
        }
    }
}

/// TODO description
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Behavior on parse errors
    #[arg(long, default_value_t = OnParseError::Warn, value_name = "CHOICE")]
    on_parse_error: OnParseError,

    /// Number of threads
    #[arg(short, long, default_value_t = num_cpus::get())]
    pub jobs: usize,

    /// Number of mutations per teset
    #[arg(short, long, default_value_t = 16)]
    pub mutations: usize,

    /// Directory to output to
    #[arg(short, long, default_value_os = "tree-splicer.out")]
    pub output: PathBuf,

    /// Seed
    #[arg(short, long, default_value_t = 0)]
    pub seed: u64,

    /// How many tests to make
    #[arg(long, default_value_t = 4)]
    pub tests: usize,

    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,

    /// Input files, use `-` to pass a single file on stdin
    #[arg(value_name = "FILE", required = true, num_args = 1..)]
    pub files: Vec<String>,
}

fn read_file(file: &str) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("Failed to read file {}", file))
}

fn parse(language: tree_sitter::Language, code: &str) -> Result<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(language)
        .context("Failed to set tree-sitter parser language")?;
    parser.parse(code, None).context("Failed to parse code")
}

#[inline]
fn stdin_string() -> Result<String> {
    let mut stdin_str: String = String::new();
    io::stdin().read_to_string(&mut stdin_str)?;
    Ok(stdin_str)
}

#[inline]
fn log_tracing_level(level: &log::Level) -> tracing::Level {
    match level {
        log::Level::Trace => tracing::Level::TRACE,
        log::Level::Debug => tracing::Level::DEBUG,
        log::Level::Info => tracing::Level::INFO,
        log::Level::Warn => tracing::Level::WARN,
        log::Level::Error => tracing::Level::ERROR,
    }
}

#[inline]
fn init_tracing(args: &Args) {
    let builder = tracing_subscriber::fmt::fmt()
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_target(false)
        .with_max_level(log_tracing_level(
            &args.verbose.log_level().unwrap_or(log::Level::Info),
        ));
    builder.event_format(formatter::TerseFormatter).init();
}

pub fn main(language: tree_sitter::Language) -> Result<()> {
    let args = Args::parse();

    init_tracing(&args);

    let mut files = HashMap::new();
    for f in args.files {
        if f == "-" {
            let path = "<stdin>".to_string();
            let s = stdin_string()?;
            let tree = parse(language, &s)?;
            handle_parse_errors(&path, &tree, &args.on_parse_error);
            files.insert(path, (s.into_bytes(), tree));
        } else {
            let path = f;
            let s = read_file(&path)?;
            let tree = parse(language, &s)?;
            handle_parse_errors(&path, &tree, &args.on_parse_error);
            files.insert(path, (s.into_bytes(), tree));
        }
    }

    let config = Config {
        // intra_splices: 10,
        inter_splices: args.mutations,
        seed: args.seed,
        tests: args.tests,
    };
    std::fs::create_dir_all(&args.output).context("Couldn't create output directory")?;
    for (i, out) in splice::splice(config, &files).enumerate() {
        std::fs::write(args.output.join(i.to_string()), out)
            .context("Couldn't save generated test case")?;
    }

    Ok(())
}
