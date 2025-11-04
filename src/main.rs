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
use zpack::package::{
    self,
    constraint::{Cmp, CmpType, Maximize, Minimize, NumOf},
    version,
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
            constraint::{Depends, IfThen, SpecOption, Value},
            outline::{PackageOutline, SpecOutline},
        },
        spec::SpecOptionValue,
    };

    let hpl_outline = PackageOutline {
        name: "hpl".into(),
        constraints: vec![
            Depends::new("blas".into()).into(),
            Depends::new("mpi".into()).into(),
            Depends::new("gcc".into()).into(),
            // Cmp {
            //     lhs: SpecOption {
            //         package_name: "openmpi".into(),
            //         option_name: "version".into(),
            //     }
            //     .into(),
            //     rhs: Value {
            //         value: SpecOptionValue::Version(
            //             package::version::Version::new("1.0.*").unwrap(),
            //         ),
            //     }
            //     .into(),
            //     op: CmpType::Equal,
            // }
            // .into(),
            // Cmp {
            //     lhs: SpecOption {
            //         package_name: "openmpi".into(),
            //         option_name: "version".into(),
            //     }
            //     .into(),
            //     rhs: Value {
            //         value: SpecOptionValue::Version(
            //             package::version::Version::new("*.0.49").unwrap(),
            //         ),
            //     }
            //     .into(),
            //     op: CmpType::Equal,
            // }
            // .into(),
        ],
        set_options: HashMap::default(),
        set_defaults: HashMap::from([(
            "static".into(),
            Some(SpecOptionValue::Bool(true)),
        )]),
    };

    let blas_outline = PackageOutline {
        name: "blas".into(),

        constraints: vec![
            Cmp {
                lhs: NumOf {
                    of: vec![
                        Cmp {
                            lhs: SpecOption {
                                package_name: "blas".into(),
                                option_name: "openblas".into(),
                            }
                            .into(),
                            rhs: Value { value: SpecOptionValue::Bool(true) }
                                .into(),
                            op: CmpType::Equal,
                        }
                        .into(),
                        Cmp {
                            lhs: SpecOption {
                                package_name: "blas".into(),
                                option_name: "mkl".into(),
                            }
                            .into(),
                            rhs: Value { value: SpecOptionValue::Bool(true) }
                                .into(),
                            op: CmpType::Equal,
                        }
                        .into(),
                    ],
                }
                .into(),
                rhs: Value { value: SpecOptionValue::Int(1) }.into(),
                op: CmpType::Equal,
            }
            .into(),
            IfThen {
                cond: Cmp {
                    lhs: SpecOption {
                        package_name: "blas".into(),
                        option_name: "openblas".into(),
                    }
                    .into(),
                    rhs: Value { value: SpecOptionValue::Bool(true) }.into(),
                    op: CmpType::Equal,
                }
                .into(),
                then: Depends::new("openblas".into()).into(),
            }
            .into(),
            IfThen {
                cond: Cmp {
                    lhs: SpecOption {
                        package_name: "blas".into(),
                        option_name: "mkl".into(),
                    }
                    .into(),
                    rhs: Value { value: SpecOptionValue::Bool(true) }.into(),
                    op: CmpType::Equal,
                }
                .into(),
                then: Depends::new("mkl".into()).into(),
            }
            .into(),
        ],

        set_options: HashMap::from([(
            "openblas".into(),
            SpecOptionValue::Bool(true),
        )]),
        set_defaults: HashMap::from([("something".into(), None)]),
    };

    let mpi_outline = PackageOutline {
        name: "mpi".into(),

        constraints: vec![
            Cmp {
                lhs: NumOf {
                    of: vec![
                        Cmp {
                            lhs: SpecOption {
                                package_name: "mpi".into(),
                                option_name: "openmpi".into(),
                            }
                            .into(),
                            rhs: Value { value: SpecOptionValue::Bool(true) }
                                .into(),
                            op: CmpType::Equal,
                        }
                        .into(),
                        Cmp {
                            lhs: SpecOption {
                                package_name: "mpi".into(),
                                option_name: "mpich".into(),
                            }
                            .into(),
                            rhs: Value { value: SpecOptionValue::Bool(true) }
                                .into(),
                            op: CmpType::Equal,
                        }
                        .into(),
                        Cmp {
                            lhs: SpecOption {
                                package_name: "mpi".into(),
                                option_name: "intelmpi".into(),
                            }
                            .into(),
                            rhs: Value { value: SpecOptionValue::Bool(true) }
                                .into(),
                            op: CmpType::Equal,
                        }
                        .into(),
                    ],
                }
                .into(),
                rhs: Value { value: SpecOptionValue::Int(1) }.into(),
                op: CmpType::Equal,
            }
            .into(),
            IfThen {
                cond: Cmp {
                    lhs: SpecOption {
                        package_name: "mpi".into(),
                        option_name: "openmpi".into(),
                    }
                    .into(),
                    rhs: Value { value: SpecOptionValue::Bool(true) }.into(),
                    op: CmpType::Equal,
                }
                .into(),
                then: Depends::new("openmpi".into()).into(),
            }
            .into(),
            IfThen {
                cond: Cmp {
                    lhs: SpecOption {
                        package_name: "mpi".into(),
                        option_name: "mpich".into(),
                    }
                    .into(),
                    rhs: Value { value: SpecOptionValue::Bool(true) }.into(),
                    op: CmpType::Equal,
                }
                .into(),
                then: Depends::new("mpich".into()).into(),
            }
            .into(),
            IfThen {
                cond: Cmp {
                    lhs: SpecOption {
                        package_name: "mpi".into(),
                        option_name: "intelmpi".into(),
                    }
                    .into(),
                    rhs: Value { value: SpecOptionValue::Bool(true) }.into(),
                    op: CmpType::Equal,
                }
                .into(),
                then: Depends::new("intelmpi".into()).into(),
            }
            .into(),
        ],

        set_options: HashMap::from([(
            "openmpi".into(),
            SpecOptionValue::Bool(true),
        )]),
        set_defaults: HashMap::default(),
    };

    let openblas_outline = PackageOutline {
        name: "openblas".into(),
        constraints: vec![Depends::new("gcc".into()).into()],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let mkl_outline = PackageOutline {
        name: "mkl".into(),
        constraints: vec![Depends::new("gcc".into()).into()],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let openmpi_versions = [
        "5.0.8",
        "5.0.7",
        "5.0.6",
        "5.0.5",
        "5.0.4",
        "5.0.3",
        "5.0.2",
        "5.0.1",
        "5.0.0",
        "4.1.8",
        "4.1.7",
        "4.1.6",
        "4.1.5",
        "4.1.4",
        "4.1.3",
        "4.1.2",
        "4.1.1",
        "4.1.0",
        "10.2.3.4.5.6.7.8.9.10",
    ]
    .into_iter()
    .map(|v| {
        Cmp {
            lhs: SpecOption {
                package_name: "openmpi".into(),
                option_name: "version".into(),
            }
            .into(),
            rhs: Value {
                value: SpecOptionValue::Version(
                    version::Version::new(v).unwrap(),
                ),
            }
            .into(),
            op: CmpType::Equal,
        }
        .into()
    })
    .collect();

    let openmpi_outline = PackageOutline {
        name: "openmpi".into(),
        constraints: vec![
            Cmp {
                lhs: NumOf { of: openmpi_versions }.into(),
                rhs: Value { value: SpecOptionValue::Int(1) }.into(),
                op: CmpType::Equal,
            }
            .into(),
            Cmp {
                lhs: SpecOption {
                    package_name: "openmpi".into(),
                    option_name: "version".into(),
                }
                .into(),
                rhs: Value {
                    value: SpecOptionValue::Version(
                        version::Version::new("*.2.>").unwrap(),
                    ),
                }
                .into(),
                op: CmpType::Less,
            }
            .into(),
            Cmp {
                lhs: SpecOption {
                    package_name: "openmpi".into(),
                    option_name: "version".into(),
                }
                .into(),
                rhs: Value {
                    value: SpecOptionValue::Version(
                        version::Version::new("*.*.8").unwrap(),
                    ),
                }
                .into(),
                op: CmpType::NotEqual,
            }
            .into(),
            Maximize {
                item: SpecOption {
                    package_name: "openmpi".into(),
                    option_name: "version".into(),
                }
                .into(),
            }
            .into(),
            Depends::new("openpmix".into()).into(),
            Depends::new("openprrte".into()).into(),
            Depends::new("hwloc".into()).into(),
            Depends::new("gcc".into()).into(),
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
        constraints: vec![Depends::new("gcc".into()).into()],
        set_options: HashMap::default(),
        set_defaults: HashMap::new(),
    };

    let intelmpi_outline = PackageOutline {
        name: "intelmpi".into(),
        constraints: vec![Depends::new("gcc".into()).into()],
        set_options: HashMap::default(),
        set_defaults: HashMap::new(),
    };

    let openpmix_outline = PackageOutline {
        name: "openpmix".into(),
        constraints: vec![Depends::new("gcc".into()).into()],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let openprrte_outline = PackageOutline {
        name: "openprrte".into(),
        constraints: vec![Depends::new("gcc".into()).into()],
        set_options: HashMap::default(),
        set_defaults: HashMap::default(),
    };

    let hwloc_versions = ["2.12.2", "2.12.1", "2.12.0"]
        .into_iter()
        .map(|v| {
            Cmp {
                lhs: SpecOption {
                    package_name: "hwloc".into(),
                    option_name: "version".into(),
                }
                .into(),
                rhs: Value {
                    value: SpecOptionValue::Version(
                        version::Version::new(v).unwrap(),
                    ),
                }
                .into(),
                op: CmpType::Equal,
            }
            .into()
        })
        .collect();

    let hwloc_outline = PackageOutline {
        name: "hwloc".into(),
        constraints: vec![
            Cmp {
                lhs: NumOf { of: hwloc_versions }.into(),
                rhs: Value { value: SpecOptionValue::Int(1) }.into(),
                op: CmpType::Equal,
            }
            .into(),
            Depends::new("gcc".into()).into(),
        ],
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

    let (optimizer, registry) = outline.gen_spec_solver().unwrap();

    println!("\n\n");

    let start = std::time::Instant::now();
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
                    registry.eval_option(package, option, &model, &registry)
                );
            }
        }
    }

    println!("elapsed: {:?}", start.elapsed());

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
}

fn test_config() {
    use config::Config;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum PackagePathConfig {
        Simple(PathBuf),
        Full(PackagePath),
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct PackagePath {
        path: PathBuf,

        #[serde(default)]
        recursive: bool,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct TestConfig {
        package_paths: HashMap<String, PackagePathConfig>,

        #[serde(default)]
        extra_package_paths: Vec<PathBuf>,
    }

    let settings = Config::builder()
        .add_source(config::File::with_name(
            "/Users/tobydavis/.config/zpack.yaml",
        ))
        .add_source(
            config::Environment::with_prefix("ZPACK")
                .try_parsing(true)
                .list_separator(":"),
        )
        .build()
        .unwrap();

    println!("Settings: {settings:#?}");

    println!("{:?}", settings.try_deserialize::<TestConfig>().unwrap());
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
    let _s = package_option.clone().into_string().unwrap();

    println!();

    // let sample = "[+thing, ~other_thing, boolean_val = true, 'string']";
    // let sample = r#"'hello, \"quoted\" world \' this is also escaped \' \t
    // '"#;
    // let sample = r#"[1, 2, 3, "hello, world", true, [123, 456], +hello]"#;
    // let sample = r#"[1, [2, 3], 4, +thingy]"#;
    // let sample = r#"thing = [1, [2, 3], 4, 5e5, "hello", true, false]"#;

    // let tokenized = zpack::spec::parse::tokenize_option(sample)?;
    // println!("Result: {tokenized:?}");
    // println!(
    //     "Result: {:?}",
    //     zpack::spec::parse::consume_spec_option(&tokenized)
    // );

    println!(
        "{:?}",
        package::version::Version::new("1.2.3-4321+alpha.*.>").unwrap()
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

    test_config();

    Ok(())
}
