use std::io;

use clap::{
    Arg, ArgAction, Command, ValueHint, crate_description, crate_version,
    value_parser,
};
use clap_complete::aot::{Generator, Shell, generate};
use pyo3::{ffi::c_str, prelude::*, types::IntoPyDict};
use saphyr::{LoadableYamlNode, Yaml, YamlEmitter};

fn build_cli() -> Command {
    Command::new("zpack")
        // .version(crate_version!())
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

fn main() {
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

    let interpreter =
        rustpython::InterpreterConfig::new().init_stdlib().interpreter();

    interpreter.enter(|vm| {
        if let Err(e) = vm.run_code_string(
            vm.new_scope_with_builtins(),
            r"
import math
print('hello')
print(math.pi)
            ",
            ".".into(),
        ) {
            println!("Error: {e:?}");
            println!("Error: {:?}", e.traceback());
        }
    });

    Python::with_gil(|py| {
        let sys = py.import("sys")?;
        let version: String = sys.getattr("version")?.extract()?;

        let locals = [("os", py.import("os")?)].into_py_dict(py)?;
        let code =
            c_str!("os.getenv('USER') or os.getenv('USERNAME') or 'Unknown'");
        let user: String = py.eval(code, None, Some(&locals))?.extract()?;

        let code = c_str!(
            "
import math
print(math.pi)
        "
        );

        let out = py.eval(code, None, Some(&locals));

        println!("Hello {user}, I'm Python {version}");

        PyResult::Ok(())
    })
    .expect("Error");

    let docs = Yaml::load_from_str("[1, 2, 3]").unwrap();
    let doc = &docs[0]; // select the first YAML document
    assert_eq!(doc[0].as_integer().unwrap(), 1); // access elements by index

    let mut out_str = String::new();
    let mut emitter = YamlEmitter::new(&mut out_str);
    emitter.dump(doc).unwrap(); // dump the YAML object to a String
}
