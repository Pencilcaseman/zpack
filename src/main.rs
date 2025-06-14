#![warn(clippy::pedantic, clippy::nursery)]

use std::io;

use clap::{
    Arg, ArgAction, Command, ValueHint, crate_description, crate_version,
    value_parser,
};
use clap_complete::aot::{Generator, Shell, generate};
use color_eyre::Result;
use saphyr::{LoadableYamlNode, Yaml, YamlEmitter};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, Style, ThemeSet},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

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

fn test_python() {
    let interpreter =
        rustpython::InterpreterConfig::new().init_stdlib().interpreter();

    interpreter.enter(|vm| {
        if let Err(e) = vm.run_code_string(
            vm.new_scope_with_builtins(),
            r"
import math
import zpack

print('hello')
print(math.pi)
            ",
            ".".into(),
        ) {
            println!("Error: {e:?}");
            println!("Error: {:?}", e.traceback());
        }
    });
}

fn test_yaml() {
    let yaml_str = r##"
zpack:
    packages:
        openmpi:
            compiler: gcc@14
            version: "5.0.5"
            options:
                - "fabrics=auto"
                - '+internal-pmix'
"##;

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

            if let Some(zpack) = doc.as_mapping_get_mut("zpack")
                && let Some(packages) = zpack.as_mapping_get_mut("packages")
                && let Some(openmpi) = packages.as_mapping_get_mut("openmpi")
                && let Some(options) = openmpi.as_mapping_get_mut("options")
            {
                println!("Options: {options:?}");

                let new_val = "+static";
                let new_val = Yaml::load_from_str(new_val)
                    .expect("Invalid temporary value")[0]
                    .clone();

                match options {
                    Yaml::Representation(cow, scalar_style, tag) => todo!(),
                    Yaml::Value(scalar) => todo!(),
                    Yaml::Sequence(yamls) => yamls.push(new_val),
                    Yaml::Mapping(linked_hash_map) => todo!(),
                    Yaml::Alias(_) => todo!(),
                    Yaml::BadValue => todo!(),
                }
            } else {
                println!("Did not find options!");
            }

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
    color_eyre::install()?;

    let matches = build_cli().get_matches();

    if let Some(generator) = matches.get_one::<Shell>("generator").copied() {
        let mut cmd = build_cli();
        eprintln!("Generating completion file for {generator}...");
        print_completions(generator, &mut cmd);
    }

    if let Some(print) = matches.subcommand_matches("print")
        && let Some(file) = print.get_one::<String>("file")
    {
        println!("File path: {file}");
    }

    test_python();
    test_yaml();

    let package_option =
        &Yaml::load_from_str("txt=\"Hello, \\\"Quoted\\\" World!\"").unwrap()
            [0];
    let s = package_option.clone().into_string().unwrap();
    let option = zpack::package::spec::parse_spec_option(&s)?;
    println!("Option: {option:?}");

    Ok(())
}
