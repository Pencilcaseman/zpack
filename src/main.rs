#![warn(clippy::pedantic, clippy::nursery)]

use std::{io, sync::Arc};

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

use rune::{
    Context, ContextError, Diagnostics, Module, Sources,
    compile::FileSourceLoader,
    diagnostics::EmitError,
    macros::test,
    runtime::{Function, Vm},
    termcolor::{ColorChoice, StandardStream},
};

// #[derive(rune::Any)]
// struct MyStruct {
//     #[rune(get, set)]
//     my_field: i32,
// }
//
// impl MyStruct {
//     #[rune::function(keep)]
//     const fn my_function(&self) -> i32 {
//         self.my_field * 10
//     }
// }

#[derive(Default, Debug, rune::Any, PartialEq, Eq)]
#[rune(constructor)]
struct External {
    #[rune(get, set)]
    suite_name: String,
    #[rune(get, set)]
    room_number: usize,
}

impl External {
    #[rune::function(keep)]
    const fn my_function(&self) -> usize {
        self.room_number * 10
    }
}

fn build_module() -> rune::support::Result<rune::Module> {
    let module = rune::Module::new();
    let mut context = rune_modules::default_context()?;
    let mut diagnostics = Diagnostics::new();

    let mut sources = rune::sources!(
        entry => {
            pub mod test_module {
                pub fn fib(n) {
                    if n < 2 {
                        n
                    } else {
                        fib(n - 1) + fib(n - 2)
                    }
                }
            }
        }
    );

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    context.install(&module)?;

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    Ok(module)
}

fn test_rune() -> rune::support::Result<()> {
    let mut context = rune_modules::default_context()?;
    let runtime = Arc::new(context.runtime()?);
    let mut diagnostics = Diagnostics::new();

    context.install(build_module()?)?;

    let mut sources = rune::sources!(
        entry => {
            pub fn main(number) {
                println!("Hello, World!");

                println!("fib({number}): {}", test_module::fib(number));
                println!("fact({number}): {}", test_module_2::fact(number));
                println!("test({number}): {}", test(number));
                println!("test_thing({number}, {number} + 1): {}", another_test::test_thing(number, number + 1));

                // Syntax error:
                number >> 1
            }
        }
    );

    let test_from_file = rune::Source::from_path("./test.rn")?;

    let test_source = rune::Source::new(
        "test_module",
        r#"
    mod test_module {
        pub fn fib(n) {
            if n < 2 {
                n
            } else {
                fib(n - 1) + fib(n - 2)
            }
        }
    }
    "#,
    );

    let source_2 = rune::Source::new(
        "test_module_2",
        r#"
    mod test_module_2 {
        pub fn fact(n) {
            if n < 2 {
                n
            } else {
                n * fact(n - 1)
            }
        }
    }
    "#,
    );

    let id = sources.insert(test_source?);
    println!("Source ID: {id:?}");

    let id2 = sources.insert(source_2?);
    println!("Source ID: {id2:?}");

    let id3 = sources.insert(test_from_file);
    println!("Source ID: {id3:?}");

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = Arc::new(result?);

    println!("Executing");
    let mut vm = Vm::new(runtime, unit);
    let output = vm.call(["main"], (6,))?;
    let res: i64 = rune::from_value(output)?;
    println!("Result: {res}");

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

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

    if let Some(print) = matches.subcommand_matches("print")
        && let Some(file) = print.get_one::<String>("file")
    {
        println!("File path: {file}");
    }

    test_yaml();

    // println!();
    // test_rune().unwrap();
    // println!();

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

    let tokenized = zpack::spec::parse::tokenize_option(sample)?;
    println!("Result: {tokenized:?}");
    println!(
        "Result: {:?}",
        zpack::spec::parse::consume_spec_option(&tokenized)
    );

    Ok(())
}
