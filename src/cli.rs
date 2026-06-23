use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "worker-runtime-host-gen",
    version,
    about = "Generate s6 service trees for the worker runtime host",
    long_about = "Validate a projects manifest, write a debug plan file, and optionally render per-project s6 services for wrangler dev --local."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Validate the manifest and write a plan file")]
    Validate(SharedArgs),
    #[command(about = "Validate, write a plan file, and render service directories")]
    Generate(GenerateArgs),
}

#[derive(Debug, Args, Clone)]
pub struct SharedArgs {
    #[arg(
        long,
        value_name = "PATH",
        default_value = "/work/host/config/projects.json",
        help = "Manifest file containing the explicit project list"
    )]
    pub manifest: PathBuf,

    #[arg(
        long,
        value_name = "DIR",
        default_value = "/etc/s6-overlay/s6-rc.d",
        help = "Output directory for generated s6-rc service source directories"
    )]
    pub output_dir: PathBuf,

    #[arg(
        long,
        value_name = "PATH",
        default_value = "/work/host/config/projects.plan.json",
        help = "Debug plan JSON output path"
    )]
    pub plan_file: PathBuf,

    #[arg(
        long,
        value_name = "DIR",
        default_value = "/run/service",
        help = "Service root used by watcher restart commands"
    )]
    pub service_root: PathBuf,

    #[arg(
        long,
        value_name = "LEVEL",
        default_value = "warn",
        help = "Wrangler log level"
    )]
    pub log_level: String,
}

#[derive(Debug, Args)]
pub struct GenerateArgs {
    #[command(flatten)]
    pub shared: SharedArgs,

    #[arg(
        long,
        help = "Write the plan file and print it, but do not render service directories"
    )]
    pub dry_run: bool,

    #[arg(
        long,
        help = "Create project runtime/state/log directories before writing service files"
    )]
    pub create_project_dirs: bool,
}
