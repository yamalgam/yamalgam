//! The yg query pipeline: inputs to filter engine to serialized output.

use std::io::{Read as _, Write};
use std::process::ExitCode;
use std::rc::Rc;

use yamalgam_core::{LoaderConfig, TagResolution};
use yamalgam_query::emit::{to_json, to_yaml};
use yamalgam_query::engine::{self, CompiledFilter, InputMeta, RunCtx};
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

    if args.in_place {
        if files.is_empty() || files.iter().any(|path| path == "-") {
            eprintln!("yg: -i/--in-place requires one or more file paths");
            return Ok(ExitCode::from(2));
        }
        return Ok(
            match run_in_place(&filter, &files, args, json_output, &config) {
                Err(error) => report(error),
                Ok(last) => exit_status(last, args.exit_status),
            },
        );
    }

    let records = match collect_inputs(&files, args, &config) {
        Ok(records) => records,
        Err(error) => return Ok(report(error)),
    };
    let context = Rc::new(RunCtx::new(config));
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let result = run_records(
        &filter,
        context,
        records,
        args,
        json_output,
        color,
        &mut output,
    );

    Ok(match result {
        Err(error) => report(error),
        Ok(last) => exit_status(last, args.exit_status),
    })
}

fn run_records(
    filter: &CompiledFilter,
    context: Rc<RunCtx>,
    records: Vec<InputRecord>,
    args: &QueryArgs,
    json_output: bool,
    color: bool,
    output: &mut dyn Write,
) -> Result<Option<bool>, YgError> {
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
    let mut first = true;
    engine::run_filter_with_null_input(
        filter,
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
    )
}

fn exit_status(last: Option<bool>, enabled: bool) -> ExitCode {
    if enabled {
        match last {
            Some(true) => ExitCode::SUCCESS,
            Some(false) => ExitCode::from(1),
            None => ExitCode::from(4),
        }
    } else {
        ExitCode::SUCCESS
    }
}

fn run_in_place(
    filter: &CompiledFilter,
    files: &[camino::Utf8PathBuf],
    args: &QueryArgs,
    json_output: bool,
    config: &LoaderConfig,
) -> Result<Option<bool>, YgError> {
    let mut last = None;

    for (file_index, path) in files.iter().enumerate() {
        let source_metadata = std::fs::metadata(path)
            .map_err(|error| YgError::InputParse(format!("failed to inspect {path}: {error}")))?;
        let mut records = collect_inputs(std::slice::from_ref(path), args, config)?;
        for record in &mut records {
            record.metadata.filename = Some(Rc::<str>::from(path.as_str()));
            record.metadata.file_index = file_index;
        }

        let parent = path.parent().unwrap_or_else(|| camino::Utf8Path::new("."));
        let mut temporary = tempfile::Builder::new()
            .prefix(".yg-")
            .tempfile_in(parent)
            .map_err(|error| {
                YgError::Runtime(format!(
                    "failed to create temporary file beside {path}: {error}"
                ))
            })?;
        let file_last = run_records(
            filter,
            Rc::new(RunCtx::new(config.clone())),
            records,
            args,
            json_output,
            false,
            temporary.as_file_mut(),
        )?;
        temporary
            .as_file_mut()
            .flush()
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|error| {
                YgError::Runtime(format!(
                    "failed to finish temporary output for {path}: {error}"
                ))
            })?;
        temporary
            .as_file()
            .set_permissions(source_metadata.permissions())
            .map_err(|error| {
                YgError::Runtime(format!(
                    "failed to preserve permissions for {path}: {error}"
                ))
            })?;
        temporary.persist(path).map_err(|error| {
            YgError::Runtime(format!(
                "failed to replace {path} atomically: {}",
                error.error
            ))
        })?;

        if file_last.is_some() {
            last = file_last;
        }
    }

    Ok(last)
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
