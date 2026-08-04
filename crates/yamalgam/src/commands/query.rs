//! The yg query pipeline: inputs to filter engine to serialized output.

use std::collections::VecDeque;
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
    value: Result<YamlVal, YgError>,
}

struct BoundVariables {
    names: Vec<String>,
    values: Vec<YamlVal>,
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
    if args.in_place && json_output {
        eprintln!("yg: -i/--in-place cannot be combined with JSON output");
        return Ok(ExitCode::from(2));
    }
    let variables = match bound_variables(args, &config) {
        Ok(variables) => variables,
        Err(error) => return Ok(report(error)),
    };
    let filter = match engine::compile_filter(&filter_source, &variables.names) {
        Ok(filter) => filter,
        Err(error) => return Ok(report(error)),
    };

    if args.in_place {
        if files.is_empty() || files.iter().any(|path| path == "-") {
            eprintln!("yg: -i/--in-place requires one or more file paths");
            return Ok(ExitCode::from(2));
        }
        return Ok(
            match run_in_place(
                &filter,
                &files,
                args,
                json_output,
                &config,
                &variables.values,
            ) {
                Err(error) => report(error),
                Ok(last) => exit_status(last, args.exit_status),
            },
        );
    }

    if !args.null_input
        && let Err(error) = validate_input_paths(&files)
    {
        return Ok(report(error));
    }
    let records = deferred_inputs(files, args.clone(), config.clone());
    let context = Rc::new(RunCtx::new(config).with_variables(variables.values));
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
    )
    .and_then(|last| {
        output.flush().map_err(output_error)?;
        Ok(last)
    });

    Ok(match result {
        Err(error) => report(error),
        Ok(last) => exit_status(last, args.exit_status),
    })
}

fn run_records(
    filter: &CompiledFilter,
    context: Rc<RunCtx>,
    records: impl Iterator<Item = InputRecord> + 'static,
    args: &QueryArgs,
    json_output: bool,
    color: bool,
    output: &mut dyn Write,
) -> Result<Option<bool>, YgError> {
    let iterator_context = Rc::clone(&context);
    let inputs = records.map(move |record| {
        iterator_context.stage_input_meta(record.metadata);
        record.value
    });

    // r[impl cli.compact]
    // r[impl cli.sort-keys]
    let options = EmitOpts {
        compact: args.compact_output,
        sort_keys: args.sort_keys,
        color: color && !json_output,
        schema: context.config.tag_resolution,
        limits: context.config.limits.clone(),
        ..EmitOpts::default()
    };
    let mut first = true;
    engine::run_filter_with_null_input(
        filter,
        &context,
        inputs,
        args.null_input,
        |value| -> Result<(), YgError> {
            // r[impl cli.json-output]
            // r[impl cli.raw-output]
            // r[impl output.multidoc]
            // r[impl output.multidoc.suppress]
            if json_output {
                writeln!(output, "{}", to_json(&value, &options)?).map_err(output_error)?;
            } else if args.raw_output && value.is_string() {
                writeln!(output, "{}", value.as_str_content().unwrap_or_default())
                    .map_err(output_error)?;
            } else {
                if !first && !args.no_doc {
                    writeln!(output, "---").map_err(output_error)?;
                }
                write!(output, "{}", to_yaml(&value, &options)?).map_err(output_error)?;
            }
            first = false;
            Ok(())
        },
    )
}

// r[impl cli.exit.success]
// r[impl cli.exit.falsy]
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

// r[impl cli.inplace]
// r[impl output.yaml.style-preservation]
fn run_in_place(
    filter: &CompiledFilter,
    files: &[camino::Utf8PathBuf],
    args: &QueryArgs,
    json_output: bool,
    config: &LoaderConfig,
    variables: &[YamlVal],
) -> Result<Option<bool>, YgError> {
    struct StagedEdit {
        path: camino::Utf8PathBuf,
        temporary: tempfile::NamedTempFile,
        last: bool,
    }

    let mut staged = Vec::with_capacity(files.len());

    for (file_index, path) in files.iter().enumerate() {
        let source_metadata = std::fs::symlink_metadata(path)
            .map_err(|error| YgError::InputRead(format!("failed to inspect {path}: {error}")))?;
        if source_metadata.file_type().is_symlink() {
            return Err(YgError::Runtime(format!(
                "refusing to replace symbolic link {path}; edit its target path directly"
            )));
        }
        warn_if_hard_linked(path, &source_metadata);
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
            Rc::new(RunCtx::new(config.clone()).with_variables(variables.iter().cloned())),
            records.into_iter(),
            args,
            json_output,
            false,
            temporary.as_file_mut(),
        )?
        .ok_or_else(|| {
            YgError::Runtime(format!(
                "{path}: filter produced no output; refusing to replace the input file"
            ))
        })?;
        temporary
            .as_file_mut()
            .flush()
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|error| {
                YgError::Runtime(format!(
                    "failed to finish temporary output for {path}: {error}"
                ))
            })?;
        preserve_owner(temporary.path(), &source_metadata, path)?;
        temporary
            .as_file()
            .set_permissions(source_metadata.permissions())
            .map_err(|error| {
                YgError::Runtime(format!(
                    "failed to preserve permissions for {path}: {error}"
                ))
            })?;
        staged.push(StagedEdit {
            path: path.clone(),
            temporary,
            last: file_last,
        });
    }

    let mut replaced = Vec::new();
    let mut last = None;
    for edit in staged {
        let path = edit.path;
        if let Err(error) = edit.temporary.persist(&path) {
            let replaced = if replaced.is_empty() {
                String::new()
            } else {
                format!("; already replaced: {}", replaced.join(", "))
            };
            return Err(YgError::Runtime(format!(
                "failed to replace {path} atomically: {}{replaced}",
                error.error
            )));
        }
        replaced.push(path.to_string());
        last = Some(edit.last);
    }

    Ok(last)
}

fn bound_variables(args: &QueryArgs, config: &LoaderConfig) -> Result<BoundVariables, YgError> {
    let mut names = Vec::new();
    let mut values = Vec::new();

    for pair in args.string_args.chunks_exact(2) {
        push_variable(
            &mut names,
            &mut values,
            &pair[0],
            YamlVal::str(pair[1].as_str()),
        )?;
    }
    for pair in args.json_args.chunks_exact(2) {
        let value = yamalgam_query::parse_json_with_config(&pair[1], config).map_err(|error| {
            YgError::Usage(format!("invalid JSON for --argjson {}: {error}", pair[0]))
        })?;
        push_variable(&mut names, &mut values, &pair[0], value)?;
    }

    Ok(BoundVariables { names, values })
}

fn push_variable(
    names: &mut Vec<String>,
    values: &mut Vec<YamlVal>,
    name: &str,
    value: YamlVal,
) -> Result<(), YgError> {
    if names.iter().any(|existing| existing == name) {
        return Err(YgError::Usage(format!(
            "variable ${name} was supplied more than once"
        )));
    }
    names.push(name.to_owned());
    values.push(value);
    Ok(())
}

#[cfg(unix)]
fn warn_if_hard_linked(path: &camino::Utf8Path, metadata: &std::fs::Metadata) {
    use std::os::unix::fs::MetadataExt as _;

    if metadata.nlink() > 1 {
        eprintln!(
            "warning: {path} has {} hard links; atomic replacement will update only this path",
            metadata.nlink()
        );
    }
}

#[cfg(not(unix))]
fn warn_if_hard_linked(_path: &camino::Utf8Path, _metadata: &std::fs::Metadata) {}

#[cfg(unix)]
fn preserve_owner(
    temporary: &std::path::Path,
    source: &std::fs::Metadata,
    path: &camino::Utf8Path,
) -> Result<(), YgError> {
    use std::os::unix::fs::MetadataExt as _;

    std::os::unix::fs::chown(temporary, Some(source.uid()), Some(source.gid())).map_err(|error| {
        YgError::Runtime(format!("failed to preserve ownership for {path}: {error}"))
    })
}

#[cfg(not(unix))]
fn preserve_owner(
    _temporary: &std::path::Path,
    _source: &std::fs::Metadata,
    _path: &camino::Utf8Path,
) -> Result<(), YgError> {
    Ok(())
}

fn filter_and_files(args: &QueryArgs) -> Result<(String, Vec<camino::Utf8PathBuf>), YgError> {
    match &args.from_file {
        Some(path) => {
            let source = std::fs::read_to_string(path).map_err(|error| {
                YgError::InputRead(format!("failed to read filter {path}: {error}"))
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

fn deferred_inputs(
    files: Vec<camino::Utf8PathBuf>,
    args: QueryArgs,
    config: LoaderConfig,
) -> impl Iterator<Item = InputRecord> {
    let mut next_file = 0;
    let mut loaded_stdin = false;
    let mut loaded_slurp = false;
    let mut records = VecDeque::new();

    std::iter::from_fn(move || {
        loop {
            if let Some(record) = records.pop_front() {
                return Some(record);
            }

            if args.slurp {
                if loaded_slurp {
                    return None;
                }
                loaded_slurp = true;
                match collect_inputs(&files, &args, &config) {
                    Ok(inputs) => records.extend(inputs),
                    Err(error) => records.push_back(InputRecord {
                        metadata: InputMeta::default(),
                        value: Err(error),
                    }),
                }
                continue;
            }

            if files.is_empty() {
                if loaded_stdin {
                    return None;
                }
                loaded_stdin = true;
                match collect_inputs(&[], &args, &config) {
                    Ok(inputs) => records.extend(inputs),
                    Err(error) => records.push_back(InputRecord {
                        metadata: InputMeta::default(),
                        value: Err(error),
                    }),
                }
                continue;
            }

            let path = files.get(next_file)?;
            let file_index = next_file;
            next_file += 1;
            match collect_inputs(std::slice::from_ref(path), &args, &config) {
                Ok(mut inputs) => {
                    for input in &mut inputs {
                        input.metadata.file_index = file_index;
                    }
                    records.extend(inputs);
                }
                Err(error) => records.push_back(InputRecord {
                    metadata: InputMeta {
                        filename: (path != "-").then(|| Rc::<str>::from(path.as_str())),
                        file_index,
                        document_index: 0,
                    },
                    value: Err(error),
                }),
            }
        }
    })
}

fn validate_input_paths(files: &[camino::Utf8PathBuf]) -> Result<(), YgError> {
    for path in files.iter().filter(|path| path.as_str() != "-") {
        std::fs::metadata(path)
            .map_err(|error| YgError::InputRead(format!("failed to read {path}: {error}")))?;
    }
    Ok(())
}

// r[impl cli.multi-file]
// r[impl cli.null-input]
// r[impl cli.slurp]
// r[impl cli.raw-input]
// r[impl ext.documents]
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
                YgError::InputRead(format!("failed to read standard input: {error}"))
            })?;
        append_input(&mut records, input, None, 0, args, config);
    } else {
        for (file_index, path) in files.iter().enumerate() {
            let input = if path == "-" {
                let mut input = String::new();
                std::io::stdin()
                    .read_to_string(&mut input)
                    .map_err(|error| {
                        YgError::InputRead(format!("failed to read standard input: {error}"))
                    })?;
                input
            } else {
                std::fs::read_to_string(path).map_err(|error| {
                    YgError::InputRead(format!("failed to read {path}: {error}"))
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
            .collect::<Result<Vec<_>, _>>()?;
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
            value: Err(YgError::InputParse(filename.map_or_else(
                || error.to_string(),
                |filename| format!("{filename}: {error}"),
            ))),
        }),
    }
}

// r[impl datamodel.schema.default]
// r[impl datamodel.schema.failsafe]
// r[impl datamodel.schema.json]
// r[impl datamodel.schema.yaml11]
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
    if !matches!(error, YgError::Halt(_) | YgError::BrokenPipe) {
        eprintln!("yg: {error}");
    }
    exit_for(&error)
}

fn output_error(error: std::io::Error) -> YgError {
    if error.kind() == std::io::ErrorKind::BrokenPipe {
        YgError::BrokenPipe
    } else {
        YgError::Runtime(format!("failed to write output: {error}"))
    }
}

// r[impl cli.exit]
// r[impl cli.exit.usage]
// r[impl cli.exit.compile]
// r[impl cli.exit.runtime]
fn exit_for(error: &YgError) -> ExitCode {
    match error {
        YgError::Usage(_) | YgError::FilterSyntax(_) => ExitCode::from(2),
        YgError::FilterCompile(_) => ExitCode::from(3),
        YgError::InputRead(_) => ExitCode::from(2),
        YgError::InputParse(_) | YgError::Runtime(_) => ExitCode::from(5),
        YgError::Halt(code) => ExitCode::from(*code as u8),
        YgError::BrokenPipe => ExitCode::SUCCESS,
    }
}
