use crate::cli::{Cli, Command};
use crate::error::Result;
use crate::plan::build_plan;
use crate::render::{render_plan, write_plan_file};
use clap::Parser;

#[must_use]
pub fn run_main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("worker-runtime-host-gen: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Runs the top-level generator command.
///
/// # Errors
///
/// Returns an error if manifest parsing, validation, plan writing, or service
/// rendering fails.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate(args) => {
            let plan = build_plan(&args, false)?;
            write_plan_file(&args.plan_file, &plan)?;
            eprintln!(
                "worker-runtime-host-gen: validated manifest and wrote plan file: {}",
                args.plan_file.display()
            );
            Ok(())
        }
        Command::Generate(args) => {
            let plan = build_plan(&args.shared, args.dry_run)?;
            write_plan_file(&args.shared.plan_file, &plan)?;

            if args.dry_run {
                eprintln!(
                    "worker-runtime-host-gen: dry run: validated manifest and wrote plan file: {}",
                    args.shared.plan_file.display()
                );
                println!("{}", serde_json::to_string_pretty(&plan)?);
                return Ok(());
            }

            eprintln!(
                "worker-runtime-host-gen: rendering services into: {}",
                args.shared.output_dir.display()
            );
            render_plan(&plan, args.create_project_dirs, &args.shared.log_level)?;
            eprintln!(
                "worker-runtime-host-gen: wrote plan file: {}",
                args.shared.plan_file.display()
            );
            Ok(())
        }
    }
}
