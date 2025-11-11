mod copilot;
mod output;
mod supermaven;
mod zed;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use output::OutputFormat;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "completion-test")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Test Zed completion provider
    Zed {
        #[clap(flatten)]
        args: CommonArgs,
    },
    /// Test Copilot completion provider
    Copilot {
        #[clap(flatten)]
        args: CommonArgs,
    },
    /// Test Supermaven completion provider
    Supermaven {
        #[clap(flatten)]
        args: CommonArgs,
    },
    /// Compare all three providers side-by-side
    Compare {
        #[clap(flatten)]
        args: CommonArgs,
        /// Timeout per provider in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
        /// Show differences between providers
        #[arg(long)]
        show_diff: bool,
    },
}

#[derive(Args, Debug, Clone)]
struct CommonArgs {
    /// File to test completions in
    #[arg(long, short)]
    file: PathBuf,
    /// Line number (0-indexed)
    #[arg(long, short)]
    line: u32,
    /// Column number (0-indexed)
    #[arg(long, short)]
    column: u32,
    /// Output format
    #[arg(long, value_enum, default_value = "human")]
    output_format: OutputFormat,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Zed { args } => zed::test_zed(args),
        Command::Copilot { args } => copilot::test_copilot(args),
        Command::Supermaven { args } => supermaven::test_supermaven(args),
        Command::Compare {
            args,
            timeout,
            show_diff,
        } => compare_all(args, timeout, show_diff),
    }
}

fn compare_all(args: CommonArgs, _timeout: u64, show_diff: bool) -> Result<()> {
    use std::time::Instant;

    let start = Instant::now();
    let mut results = Vec::new();

    // Test all providers
    let zed_result = zed::test_zed_internal(args.clone());
    let zed_duration = start.elapsed();
    let start2 = Instant::now();
    
    let copilot_result = copilot::test_copilot_internal(args.clone());
    let copilot_duration = start2.elapsed();
    let start3 = Instant::now();
    
    let supermaven_result = supermaven::test_supermaven_internal(args.clone());
    let supermaven_duration = start3.elapsed();

    results.push(("zed", zed_result, zed_duration));
    results.push(("copilot", copilot_result, copilot_duration));
    results.push(("supermaven", supermaven_result, supermaven_duration));

    match args.output_format {
        OutputFormat::Human | OutputFormat::Table => {
            output::print_comparison(&results, show_diff)
        }
        OutputFormat::Json => output::print_comparison_json(&results),
    }
}

