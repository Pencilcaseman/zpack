#![warn(clippy::pedantic, clippy::nursery)]

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

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
use z3::{Optimize, PrepareSynchronized};
use zpack::{
    package::constraint::n_of::NOf,
    spec::spec_option::{SpecOption, SpecOptionType},
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

#[allow(clippy::too_many_lines)]
fn test_outline() {
    use std::collections::HashMap;

    use zpack::{
        package::{
            constraint::{
                Constraint, depends::Depends, if_then::IfThen,
                spec_option::SpecOptionEqual,
            },
            outline::*,
        },
        spec::spec_option::SpecOptionValue,
    };

    let hpl_outline = PackageOutline {
        name: "hpl".into(),
        constraints: vec![
            Box::new(Depends("blas".into())),
            Box::new(Depends("mpi".into())),
            Box::new(Depends("gcc".into())),
            Box::new(SpecOptionEqual {
                package_name: Some("mpi".into()),
                option_name: "openmpi_off".into(),
                equal_to: SpecOptionValue::Bool(false),
            }),
        ],
        set_options: HashMap::default(),
        set_defaults: HashMap::from([
            ("static".into(), Some(SpecOptionValue::Bool(true))),
            ("something".into(), None),
        ]),
    };

    let blas_outline = PackageOutline {
        name: "blas".into(),

        constraints: vec![Box::new(NOf {
            n: 1,
            of: vec![
                Box::new(IfThen {
                    cond: Box::new(SpecOptionEqual {
                        package_name: None,
                        option_name: "openblas".into(),
                        equal_to: SpecOptionValue::Bool(true),
                    }),
                    then: Box::new(Depends("openblas".into())),
                }),
                Box::new(IfThen {
                    cond: Box::new(SpecOptionEqual {
                        package_name: None,
                        option_name: "mkl".into(),
                        equal_to: SpecOptionValue::Bool(true),
                    }),
                    then: Box::new(Depends("mkl".into())),
                }),
            ],
        })],

        set_options: HashMap::from([(
            "openblas".into(),
            SpecOptionValue::Bool(true),
        )]),
        set_defaults: HashMap::default(),
    };

    let mpi_outline = PackageOutline {
        name: "mpi".into(),

        constraints: vec![Box::new(NOf {
            n: 1,
            of: vec![
                Box::new(IfThen {
                    cond: Box::new(SpecOptionEqual {
                        package_name: None,
                        option_name: "openmpi".into(),
                        equal_to: SpecOptionValue::Bool(true),
                    }),
                    then: Box::new(Depends("openmpi".into())),
                }),
                Box::new(IfThen {
                    cond: Box::new(SpecOptionEqual {
                        package_name: None,
                        option_name: "mpich".into(),
                        equal_to: SpecOptionValue::Bool(true),
                    }),
                    then: Box::new(Depends("mpich".into())),
                }),
                Box::new(IfThen {
                    cond: Box::new(SpecOptionEqual {
                        package_name: None,
                        option_name: "intelmpi".into(),
                        equal_to: SpecOptionValue::Bool(true),
                    }),
                    then: Box::new(Depends("intelmpi".into())),
                }),
            ],
        })],

        set_options: HashMap::from([(
            "openmpi".into(),
            SpecOptionValue::Bool(true),
        )]),
        set_defaults: HashMap::default(),
    };

    let openblas_outline = PackageOutline {
        name: "openblas".into(),
        constraints: vec![Box::new(Depends("gcc".into()))],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let mkl_outline = PackageOutline {
        name: "mkl".into(),
        constraints: vec![Box::new(Depends("gcc".into()))],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let openmpi_outline = PackageOutline {
        name: "openmpi".into(),
        constraints: vec![
            Box::new(Depends("openpmix".into())),
            Box::new(Depends("openprrte".into())),
            Box::new(Depends("hwloc".into())),
            Box::new(Depends("gcc".into())),
        ],
        set_options: HashMap::default(),
        set_defaults: HashMap::from([
            // ("static".into(), None),
            // ("static".into(), Some(SpecOptionValue::Bool(false))),
            ("fabrics".into(), Some(SpecOptionValue::Str("auto".into()))),
        ]),
    };

    let mpich_outline = PackageOutline {
        name: "mpich".into(),
        constraints: vec![Box::new(Depends("gcc".into()))],
        set_options: HashMap::default(),
        set_defaults: HashMap::new(),
    };

    let intelmpi_outline = PackageOutline {
        name: "intelmpi".into(),
        constraints: vec![Box::new(Depends("gcc".into()))],
        set_options: HashMap::default(),
        set_defaults: HashMap::new(),
    };

    let openpmix_outline = PackageOutline {
        name: "openpmix".into(),
        constraints: vec![Box::new(Depends("gcc".into()))],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let openprrte_outline = PackageOutline {
        name: "openprrte".into(),
        constraints: vec![Box::new(Depends("gcc".into()))],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let hwloc_outline = PackageOutline {
        name: "hwloc".into(),
        constraints: vec![Box::new(Depends("gcc".into()))],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let gcc_outline = PackageOutline {
        name: "gcc".into(),
        constraints: Vec::new(),
        set_options: HashMap::default(),
        set_defaults: HashMap::from([(
            "static".into(),
            Some(SpecOptionValue::Bool(true)),
        )]),
    };

    let outlines = vec![
        blas_outline,
        gcc_outline,
        hpl_outline,
        hwloc_outline,
        intelmpi_outline,
        mkl_outline,
        mpi_outline,
        mpich_outline,
        openblas_outline,
        openmpi_outline,
        openpmix_outline,
        openprrte_outline,
    ];

    let mut outline = SpecOutline::new(outlines).unwrap();
    outline.required.push("hpl".to_string());

    outline.propagate_defaults().unwrap();

    let mut config = z3::Config::new();
    config.set_bool_param_value("unsat_core", true);

    let (optimizer, vars) = outline.gen_spec_solver().unwrap();

    println!("\n\n");

    match optimizer.check(&[]) {
        z3::SatResult::Unsat => {
            tracing::info!("unsat");

            println!("No solution found.");
            // println!("Proof: {:?}", optimizer.get_proof());
            println!("UnsatCore: {:?}", optimizer.get_unsat_core());

            println!("Conflicting Constraints:");
            for lit in optimizer.get_unsat_core() {
                println!("- {lit:?}");
            }
        }
        z3::SatResult::Unknown => {
            tracing::info!("unknown");
            todo!();
        }
        z3::SatResult::Sat => {
            tracing::info!("sat");

            let model = optimizer.get_model().unwrap();

            for (k, v) in vars {
                println!("{k:?} -> {:?}", model.eval(&v, true));
            }
        }
    }

    println!("\n\n");

    // let mut child = std::process::Command::new("dot")
    //     .arg("-Tsvg")
    //     .arg("-o")
    //     .arg("output.svg")
    //     .stdin(std::process::Stdio::piped())
    //     .spawn()
    //     .expect("Failed to spawn dot process");
    //
    // if let Some(mut stdin) = child.stdin.take() {
    //     write!(
    //         stdin,
    //         "{}",
    //         petgraph::dot::Dot::with_config(
    //             &outline.graph,
    //             &[petgraph::dot::Config::EdgeNoLabel]
    //         )
    //     )
    //     .expect("Failed to write to stdin");
    // }
    //
    // child.wait().expect("dot command failed");

    println!("TopoSort: {:?}", petgraph::algo::toposort(&outline.graph, None));

    for idx in petgraph::algo::toposort(&outline.graph, None).unwrap() {
        println!(
            "{}: {:?}",
            outline.graph[idx].name, outline.graph[idx].set_defaults
        );
    }
}

fn test_z3() {
    use z3::{
        Config, Context, SatResult, Solver,
        ast::{Ast, Bool, Int},
    };

    let mut config = Config::new();
    config.set_bool_param_value("proof", true);
    config.set_bool_param_value("unsat_core", true);

    z3::with_z3_config(&config, || {
        // Create a solver instance.
        let solver = Solver::new();

        // Create integer variables for each letter.
        let s = Int::new_const("S");
        let e = Int::new_const("E");
        let n = Int::new_const("N");
        let d = Int::new_const("D");
        let m = Int::new_const("M");
        let o = Int::new_const("O");
        let r = Int::new_const("R");
        let y = Int::new_const("Y");

        let letters = [&s, &e, &n, &d, &m, &o, &r, &y];

        // Add constraints:
        // 1. Each letter must be a digit between 0 and 9.
        for letter in &letters {
            solver.assert_and_track(
                letter.ge(Int::from_i64(0)),
                &Bool::new_const(format!("{letter} >= 0")),
            );

            solver.assert_and_track(
                letter.le(Int::from_i64(9)),
                &Bool::new_const(format!("{letter} <= 9")),
            );
        }

        // 2. All letters must have distinct values &letters;

        // 3. The leading letters S and M cannot be zero.
        solver.assert_and_track(
            s.ne(Int::from_i64(0)),
            &Bool::new_const("S != 0"),
        );

        solver.assert_and_track(
            m.ne(Int::from_i64(0)),
            &Bool::new_const("M != 0"),
        );

        // 4. The equation SEND + MORE = MONEY must hold.
        // This is expressed in terms of the numerical value of the words.
        let send = &s * 1000 + &e * 100 + &n * 10 + &d;
        let more = &m * 1000 + &o * 100 + &r * 10 + &e;
        let money = &m * 10000 + &o * 1000 + &n * 100 + &e * 10 + &y;

        solver.assert_and_track(
            (send + more).eq(&money),
            &Bool::new_const("SEND + MORE = MONEY"),
        );

        // Check for a solution.
        tracing::info!("Check");
        let start = std::time::Instant::now();
        let c = solver.check();
        println!("SAT/UNSAT in {:?}", start.elapsed());
        match c {
            SatResult::Sat => {
                tracing::info!("SAT");
                // If a solution is found, get the model.
                let model = solver.get_model().unwrap();
                println!("Solution found:");
                for letter in &letters {
                    println!(
                        "{}: {}",
                        letter,
                        model.eval(*letter, true).unwrap()
                    );
                }
            }
            SatResult::Unsat => {
                tracing::info!("UNSAT");

                println!("No solution found.");
                println!("Proof: {:?}", solver.get_proof());
                println!("UnsatCore: {:?}", solver.get_unsat_core());

                println!("Conflicting Constraints:");
                for lit in solver.get_unsat_core() {
                    println!("- {lit:?}");
                }
            }
            SatResult::Unknown => {
                println!("Unknown");
            }
        }
    });
}

fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        zpack::util::subscriber::subscriber(),
    )?;

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

    println!(
        "{:?}",
        zpack::package::version::semver::SemVer::new("1.2.3-4321")?
    );

    let test_graph = petgraph::graph::DiGraph::<i32, ()>::from_edges([
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
    ]);

    println!("Test Graph: {test_graph:?}");
    println!("Cycle: {}", petgraph::algo::is_cyclic_directed(&test_graph));
    println!("{:?}", petgraph::dot::Dot::new(&test_graph));

    test_outline();
    test_z3();

    Ok(())
}
