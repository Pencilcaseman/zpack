#[derive(Debug, Clone, Copy)]
pub enum CliError {
    Idfk,
}

use std::path::PathBuf;

use anstyle::AnsiColor;
use clap::{
    Arg, ArgAction, Command, ValueHint, builder::styling::Styles,
    crate_description, crate_version, value_parser,
};
use clap_complete::{
    Generator,
    aot::{Shell, generate},
};
use pyo3::prelude::*;

use crate::package::outline::{PackageOutline, SpecOutline};

fn build_cli() -> Command {
    Command::new("zpack")
        .long_version(format!("{}\n{}", crate_version!(), crate_description!()))
        .arg(
            Arg::new("test")
                .short('t')
                .help("test a package config file")
                .value_parser(value_parser!(PathBuf))
                .value_hint(ValueHint::FilePath),
        )
        .subcommand(
            Command::new("print").about("Print something").arg(
                Arg::new("file")
                    .short('f')
                    .help("Input file")
                    .value_hint(ValueHint::ExecutablePath),
            ),
        )
        .arg(
            Arg::new("generator")
                .long("generate")
                .action(ArgAction::Set)
                .value_parser(value_parser!(Shell)),
        )
        .styles(
            Styles::styled()
                .usage(AnsiColor::Green.on_default().bold())
                .header(AnsiColor::BrightBlue.on_default().bold())
                .literal(AnsiColor::BrightCyan.on_default().bold())
                .placeholder(AnsiColor::BrightCyan.on_default())
                .context(AnsiColor::Yellow.on_default()),
        )
}

fn print_completions<G: Generator>(generator: G, cmd: &mut Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut std::io::stdout(),
    );
}

/// # Panics
/// Because I haven't finished this yet
fn parse<I, T>(args: I)
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let matches = build_cli().get_matches_from(args);

    if let Some(path) = matches.get_one::<PathBuf>("test") {
        println!("Testing {}", path.display());

        Python::attach(|py| {
            let packages =
                crate::interface::reader::process_file(py, path).unwrap();
            let mut outlines = Vec::new();

            for package in packages {
                let outline: PackageOutline =
                    crate::interface::reader::read_from_class0(
                        package, "outline",
                    )
                    .unwrap();

                println!("{outline:?}");
                outlines.push(outline);
            }

            let mut outline = SpecOutline::new(outlines).unwrap();
            outline.required.push("hpl".to_string());

            outline.propagate_defaults().unwrap();

            let (optimizer, registry) = outline.gen_spec_solver().unwrap();

            println!("\n\n");

            println!("Optimizer: {optimizer}");
            println!("Registry: {registry:#?}");

            println!("\n\n");

            match optimizer.check(&[]) {
                z3::SatResult::Unsat => {
                    tracing::info!("unsat");

                    println!("Conflicting Constraints:");
                    for lit in optimizer.get_unsat_core() {
                        println!(
                            "- {}",
                            registry
                                .constraint_description(&lit)
                                .cloned()
                                .unwrap_or_else(|| lit.to_string())
                        );
                    }
                }
                z3::SatResult::Unknown => {
                    tracing::info!("unknown");
                    todo!();
                }
                z3::SatResult::Sat => {
                    tracing::info!("sat");

                    let model = optimizer.get_model().unwrap();
                    for &(package, option) in registry.spec_option_names() {
                        println!(
                            "{}:{:?} -> {:?}",
                            package,
                            option,
                            registry.eval_option(
                                package, option, &model, &registry
                            )
                        );
                    }
                }
            }
        });
    } else if let Some(generator) =
        matches.get_one::<Shell>("generator").copied()
    {
        let mut cmd = build_cli();
        eprintln!("Generating completion file for {generator}...");
        print_completions(generator, &mut cmd);
    }
}

/// Main entrypoint into zpack.
///
/// # Errors
/// Errors produced during parsing, solving, building, etc. will be, in one way
/// or another, returned here.
pub fn entry(is_python: bool) -> Result<(), CliError> {
    let args = std::env::args().skip(usize::from(is_python));
    parse(args);

    Ok(())
}
