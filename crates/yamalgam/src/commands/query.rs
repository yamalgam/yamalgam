//! The yg query pipeline: inputs to filter engine to serialized output.

use std::io::{Read as _, Write as _};
use std::process::ExitCode;
use std::rc::Rc;

use yamalgam_core::{LoaderConfig, TagResolution};
use yamalgam_query::emit::{to_json, to_yaml};
use yamalgam_query::engine::{self, InputMeta, RunCtx};
use yamalgam_query::{EmitOpts, YamlVal, YgError};

use crate::{QueryArgs, SchemaArg};

struct InputRecord {
    metadata: InputMeta,
    value: Result<YamlVal, String>,
}

/// Execute yg's default query mode.
pub fn cmd_query(
    args: &QueryArgs,
    json_output: bool,
    color: bool,
    quiet: bool,
) -> anyhow::Result<ExitCode> {
    if args.in_place {
        eprintln!("yg: -i/--in-place is not yet implemented");
        return Ok(ExitCode::from(2));
    }

    let config = loader_config(args);
    if args.schema == SchemaArg::Yaml11 && !quiet {
        eprintln!("warning: YAML 1.1 schema compatibility mode resolves yes/no/on/off as booleans");
    }

    let (filter_source, files) = match filter_and_files(args) {
        Ok(parts) => parts,
        Err(error) => return Ok(report(error)),
    };
    let filter = match engine::compile_filter(&filter_source, &[]) {
        Ok(filter) => filter,
        Err(error) => return Ok(report(error)),
    };

    let records = match collect_inputs(&files, args, &config) {
        Ok(records) => records,
        Err(error) => return Ok(report(error)),
    };
    let context = Rc::new(RunCtx::new(config));
    let iterator_context = Rc::clone(&context);
    let inputs = records.into_iter().map(move |record| {
        *iterator_context.input_meta.borrow_mut() = record.metadata;
        record.value
    });

    let options = EmitOpts {
        compact: args.compact_output,
        sort_keys: args.sort_keys,
        color: color && !json_output,
        ..EmitOpts::default()
    };
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let mut first = true;
    let result = engine::run_filter_with_null_input(
        &filter,
        &context,
        inputs,
        args.null_input,
        |value| -> Result<(), YgError> {
            if json_output {
                writeln!(output, "{}", to_json(&value, &options))
                    .map_err(|error| YgError::Runtime(error.to_string()))?;
            } else if args.raw_output && value.is_string() {
                writeln!(output, "{}", value.as_str_content().unwrap_or_default())
                    .map_err(|error| YgError::Runtime(error.to_string()))?;
            } else {
                if !first && !args.no_doc {
                    writeln!(output, "---").map_err(|error| YgError::Runtime(error.to_string()))?;
                }
                write!(output, "{}", to_yaml(&value, &options))
                    .map_err(|error| YgError::Runtime(error.to_string()))?;
            }
            first = false;
            Ok(())
        },
    );

    match result {
        Err(error) => Ok(report(error)),
        Ok(last) if args.exit_status => Ok(match last {
            Some(true) => ExitCode::SUCCESS,
            Some(false) => ExitCode::from(1),
            None => ExitCode::from(4),
        }),
        Ok(_) => Ok(ExitCode::SUCCESS),
    }
}

fn filter_and_files(args: &QueryArgs) -> Result<(String, Vec<camino::Utf8PathBuf>), YgError> {
    match &args.from_file {
        Some(path) => {
            let source = std::fs::read_to_string(path).map_err(|error| {
                YgError::InputParse(format!("failed to read filter {path}: {error}"))
            })?;
            let mut files = Vec::new();
            files.extend(args.filter.clone().map(camino::Utf8PathBuf::from));
            files.extend(args.files.iter().cloned());
            Ok((source, files))
        }
        None => Ok((
            args.filter.clone().unwrap_or_else(|| ".".into()),
            args.files.clone(),
        )),
    }
}

fn collect_inputs(
    files: &[camino::Utf8PathBuf],
    args: &QueryArgs,
    config: &LoaderConfig,
) -> Result<Vec<InputRecord>, YgError> {
    let mut records = Vec::new();
    if files.is_empty() {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| {
                YgError::InputParse(format!("failed to read standard input: {error}"))
            })?;
        append_input(&mut records, input, None, 0, args, config);
    } else {
        for (file_index, path) in files.iter().enumerate() {
            let input = if path == "-" {
                let mut input = String::new();
                std::io::stdin()
                    .read_to_string(&mut input)
                    .map_err(|error| {
                        YgError::InputParse(format!("failed to read standard input: {error}"))
                    })?;
                input
            } else {
                std::fs::read_to_string(path).map_err(|error| {
                    YgError::InputParse(format!("failed to read {path}: {error}"))
                })?
            };
            append_input(
                &mut records,
                input,
                (path != "-").then(|| Rc::<str>::from(path.as_str())),
                file_index,
                args,
                config,
            );
        }
    }

    if args.slurp {
        let values = records
            .into_iter()
            .map(|record| record.value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(YgError::InputParse)?;
        let value = if args.raw_input {
            YamlVal::str(
                values
                    .iter()
                    .filter_map(YamlVal::as_str_content)
                    .collect::<String>(),
            )
        } else {
            YamlVal::seq(values)
        };
        return Ok(vec![InputRecord {
            metadata: InputMeta::default(),
            value: Ok(value),
        }]);
    }
    Ok(records)
}

fn append_input(
    records: &mut Vec<InputRecord>,
    input: String,
    filename: Option<Rc<str>>,
    file_index: usize,
    args: &QueryArgs,
    config: &LoaderConfig,
) {
    if args.raw_input {
        let values = if args.slurp {
            vec![YamlVal::str(input)]
        } else {
            input.lines().map(YamlVal::str).collect()
        };
        records.extend(
            values
                .into_iter()
                .enumerate()
                .map(|(document_index, value)| InputRecord {
                    metadata: InputMeta {
                        filename: filename.clone(),
                        file_index,
                        document_index,
                    },
                    value: Ok(value),
                }),
        );
        return;
    }

    match yamalgam_query::parse_str_with_config(&input, config) {
        Ok(values) => {
            records.extend(
                values
                    .into_iter()
                    .enumerate()
                    .map(|(document_index, value)| InputRecord {
                        metadata: InputMeta {
                            filename: filename.clone(),
                            file_index,
                            document_index,
                        },
                        value: Ok(value),
                    }),
            );
        }
        Err(error) => records.push(InputRecord {
            metadata: InputMeta {
                filename: filename.clone(),
                file_index,
                document_index: 0,
            },
            value: Err(filename.map_or_else(
                || error.to_string(),
                |filename| format!("{filename}: {error}"),
            )),
        }),
    }
}

const fn loader_config(args: &QueryArgs) -> LoaderConfig {
    let config = if args.strict {
        LoaderConfig::strict()
    } else if args.trusted {
        LoaderConfig::trusted()
    } else {
        LoaderConfig::moderate()
    };
    config.with_tag_resolution(match args.schema {
        SchemaArg::Core => TagResolution::Yaml12,
        SchemaArg::Failsafe => TagResolution::Failsafe,
        SchemaArg::Json => TagResolution::Json,
        SchemaArg::Yaml11 => TagResolution::Yaml11,
    })
}

fn report(error: YgError) -> ExitCode {
    eprintln!("yg: {error}");
    exit_for(&error)
}

fn exit_for(error: &YgError) -> ExitCode {
    match error {
        YgError::FilterSyntax(_) => ExitCode::from(2),
        YgError::FilterCompile(_) => ExitCode::from(3),
        YgError::InputParse(_) | YgError::Runtime(_) => ExitCode::from(5),
        YgError::Halt(code) => ExitCode::from((*code).clamp(0, 255) as u8),
    }
}
