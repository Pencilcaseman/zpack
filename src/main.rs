#![warn(clippy::pedantic, clippy::nursery)]

use std::io;

use anyhow::Result;
use clap::{
    Arg, ArgAction, Command, ValueHint, crate_description, crate_version,
    value_parser,
};
use clap_complete::aot::{Generator, Shell, generate};
use saphyr::{LoadableYamlNode, Yaml, YamlEmitter};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, Style, ThemeSet},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};
use tracing::instrument;

fn build_cli() -> Command {
    Command::new("zpack")
        .long_version(format!("{}\n{}", crate_version!(), crate_description!()))
        .arg(
            Arg::new("file")
                .short('f')
                .help("some input file")
                .value_hint(ValueHint::AnyPath),
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
}

fn print_completions<G: Generator>(generator: G, cmd: &mut Command) {
    generate(generator, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[instrument]
fn test_yaml() {
    let yaml_str = r#"
zpack:
    packages:
        openmpi:
            compiler: gcc@14
            version: "5.0.5"
            options:
                - "fabrics=auto"
                - '+internal-pmix'
"#;

    match Yaml::load_from_str(yaml_str) {
        Ok(mut docs) => {
            let doc = &mut docs[0]; // select the first YAML document

            if let Some(yaml) = doc.as_mapping_get("zpack") {
                println!("Info: {yaml:?}");
            }

            let mut out_str = String::new();
            let mut emitter = YamlEmitter::new(&mut out_str);
            emitter.dump(doc).unwrap(); // dump the YAML object to a String
            println!("Output string: {out_str}");

            // if let Some(zpack) = doc.as_mapping_get_mut("zpack")
            //     && let Some(packages) = zpack.as_mapping_get_mut("packages")
            //     && let Some(openmpi) = packages.as_mapping_get_mut("openmpi")
            //     && let Some(options) = openmpi.as_mapping_get_mut("options")
            // {
            //     println!("Options: {options:?}");
            //
            //     let new_val = "+static";
            //     let new_val = Yaml::load_from_str(new_val)
            //         .expect("Invalid temporary value")[0]
            //         .clone();
            //
            //     match options {
            //         Yaml::Representation(_, _, _) => todo!(),
            //         Yaml::Value(_) => todo!(),
            //         Yaml::Sequence(yamls) => yamls.push(new_val),
            //         Yaml::Mapping(_) => todo!(),
            //         Yaml::Alias(_) => todo!(),
            //         Yaml::BadValue => todo!(),
            //     }
            // } else {
            //     println!("Did not find options!");
            // }

            let mut out_str = String::new();
            let mut emitter = YamlEmitter::new(&mut out_str);
            emitter.dump(doc).unwrap(); // dump the YAML object to a String
            println!("Output string: {out_str}");
        }

        Err(err) => {
            // Load these once at the start of your program
            let ps = SyntaxSet::load_defaults_newlines();
            let ts = ThemeSet::load_defaults();

            let reference = ps
                .find_syntax_by_extension("rs")
                .expect("Unknown file extension");

            let mut theme = ts.themes["base16-ocean.dark"].clone();

            theme.settings.background =
                Some(Color { r: 255, g: 0, b: 0, a: 0 });

            let mut h = HighlightLines::new(reference, &theme);

            for line in LinesWithEndings::from(yaml_str) {
                let ranges: Vec<(Style, &str)> =
                    h.highlight_line(line, &ps).unwrap();
                let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                print!("{escaped}");
            }

            println!("Error: {err:?}");
        }
    }
}

fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("Debug Information");
    tracing::warn!("Warning Information");

    let thing = "Hello, World!";
    let things: Vec<usize> = thing.char_indices().map(|(idx, _)| idx).collect();
    println!("Thing:  {thing}");
    println!("Things: {things:?}");

    let matches = build_cli().get_matches();

    if let Some(generator) = matches.get_one::<Shell>("generator").copied() {
        let mut cmd = build_cli();
        eprintln!("Generating completion file for {generator}...");
        print_completions(generator, &mut cmd);
    }

    // if let Some(print) = matches.subcommand_matches("print")
    //     && let Some(file) = print.get_one::<String>("file")
    // {
    //     println!("File path: {file}");
    // }

    test_yaml();

    let package_option =
        &Yaml::load_from_str(r#"txt="Hello, \"Quoted\" World!""#).unwrap()[0];
    let s = package_option.clone().into_string().unwrap();

    println!();

    // let sample = "[+thing, ~other_thing, boolean_val = true, 'string']";
    // let sample = r#"'hello, \"quoted\" world \' this is also escaped \' \t
    // '"#;
    // let sample = r#"[1, 2, 3, "hello, world", true, [123, 456], +hello]"#;
    // let sample = r#"[1, [2, 3], 4, +thingy]"#;
    let sample = r#"thing = [1, [2, 3], 4, 5e5, "hello", true, false]"#;

    // let tokenized = zpack::spec::parse::tokenize_option(sample)?;
    // println!("Result: {tokenized:?}");
    // println!(
    //     "Result: {:?}",
    //     zpack::spec::parse::consume_spec_option(&tokenized)
    // );

    println!("{:?}", zpack::package::version::Version::new("1.2.3-4321",)?);

    Ok(())
}
