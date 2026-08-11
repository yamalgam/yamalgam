//! The yg query pipeline: inputs to filter engine to serialized output.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::io::{BufWriter, Read as _, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

use yamalgam_core::{LoaderConfig, ResolutionMode, Severity, TagResolution};
use yamalgam_query::emit::emit;
use yamalgam_query::engine::{self, CompiledFilter, InputMeta, RunCtx};
use yamalgam_query::{
    EmitFormat, EmitOpts, Meta, QuerySchemaResolver, ResolutionSessionKey, ResolvedInput,
    ResourceOrigin, ValidationFailure, YamlVal, YgError, resolution_handle, resolve_documents,
};
use yamalgam_resolve::{
    FileRetriever, LibrebarRetriever, ResourceId, ResourceRetriever, RetrieveError, RetrieverRouter,
};
use yamalgam_schema::{
    SchemaOptions, SchemaSession, SchemaValidator, ValidationDiagnostic, ValidationReport,
};

use crate::{QueryArgs, RefSchemeArg, SchemaArg};

struct InputRecord {
    metadata: InputMeta,
    value: Result<YamlVal, YgError>,
    resolution: Option<ResolutionSessionKey>,
}

struct ResolutionRuntime {
    http: Option<Arc<LibrebarRetriever>>,
}

struct QueryValidationResolver {
    root: Option<PathBuf>,
    session: SchemaSession,
    validators: Mutex<HashMap<ResourceId, Arc<SchemaValidator>>>,
}

struct ResolutionSource {
    base: ResourceId,
    config: LoaderConfig,
    retriever: Arc<RetrieverRouter>,
}

struct BoundVariables {
    names: Vec<String>,
    values: Vec<YamlVal>,
}

/// Resolved query output format and its format-specific settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    /// YAML output, optionally ANSI-colored.
    Yaml {
        /// Wrap supported scalar classes in ANSI color escapes.
        color: bool,
    },
    /// JSON output without YAML presentation or color.
    Json,
}

impl OutputMode {
    const fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }

    const fn color(self) -> bool {
        match self {
            Self::Yaml { color } => color,
            Self::Json => false,
        }
    }

    const fn emit_format(self) -> EmitFormat {
        match self {
            Self::Yaml { .. } => EmitFormat::Yaml,
            Self::Json => EmitFormat::Json,
        }
    }
}

/// Resolved settings for one query command invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunOptions {
    /// Output format and its format-specific settings.
    pub output: OutputMode,
    /// Suppress non-error diagnostics.
    pub quiet: bool,
}

/// Execute yg's default query mode.
pub fn cmd_query(args: &QueryArgs, options: RunOptions) -> anyhow::Result<ExitCode> {
    let config = match loader_config(args) {
        Ok(config) => config,
        Err(error) => return Ok(report(error)),
    };
    if args.schema == SchemaArg::Yaml11 && !options.quiet {
        eprintln!("warning: YAML 1.1 schema compatibility mode resolves yes/no/on/off as booleans");
    }

    let (filter_source, files) = match filter_and_files(args) {
        Ok(parts) => parts,
        Err(error) => return Ok(report(error)),
    };
    if args.in_place && options.output.is_json() {
        eprintln!("yg: -i/--in-place cannot be combined with JSON output");
        return Ok(ExitCode::from(2));
    }
    let variables = match bound_variables(args, &config) {
        Ok(variables) => variables,
        Err(error) => return Ok(report(error)),
    };
    let compile_options = if args.no_env {
        engine::CompileOptions::sandboxed()
    } else {
        engine::CompileOptions::default()
    };
    let filter = match engine::compile_filter_with_options(
        &filter_source,
        &variables.names,
        compile_options,
    ) {
        Ok(filter) => filter,
        Err(error) => return Ok(report(error)),
    };
    let runtime = match ResolutionRuntime::new(&config) {
        Ok(runtime) => runtime,
        Err(error) => return Ok(report(error)),
    };
    let validation = match QueryValidationResolver::new(&config, &runtime) {
        Ok(validation) => Arc::new(validation),
        Err(error) => return Ok(report(error)),
    };
    let context = RunCtx::new(config)
        .with_variables(variables.values)
        .with_schema_resolver(validation);

    if args.null_input {
        match anonymous_resolution(args, &context, &runtime) {
            Ok(resolution) => context.stage_input_resolution(resolution),
            Err(error) => return Ok(report(error)),
        }
    }

    if args.in_place {
        if files.is_empty() || files.iter().any(|path| path == "-") {
            eprintln!("yg: -i/--in-place requires one or more file paths");
            return Ok(ExitCode::from(2));
        }
        return Ok(
            match run_in_place(&filter, &files, args, options.output, &context, &runtime) {
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
    let records = deferred_inputs(&files, args, &context, &runtime);
    let stdout = std::io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    let result = run_records(
        &filter,
        &context,
        records,
        args,
        options.output,
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

fn run_records<'inputs>(
    filter: &CompiledFilter,
    context: &'inputs RunCtx,
    records: impl Iterator<Item = InputRecord> + 'inputs,
    args: &QueryArgs,
    output_mode: OutputMode,
    output: &mut dyn Write,
) -> Result<Option<bool>, YgError> {
    let inputs = records.map(move |record| {
        context.stage_input_meta(record.metadata);
        context.stage_input_resolution(record.resolution);
        record.value
    });

    // r[impl cli.compact]
    // r[impl cli.sort-keys]
    let mut options = EmitOpts::default();
    options.compact = args.compact_output;
    options.sort_keys = args.sort_keys;
    options.color = output_mode.color();
    options.schema = context.config.tag_resolution;
    options.limits = context.config.limits.clone();
    let mut first = true;
    engine::run_filter_with_null_input(
        filter,
        context,
        inputs,
        args.null_input,
        |value| -> Result<(), YgError> {
            // r[impl cli.json-output]
            // r[impl cli.raw-output]
            // r[impl output.multidoc]
            // r[impl output.multidoc.suppress]
            if output_mode.is_json() {
                writeln!(
                    output,
                    "{}",
                    emit(&value, output_mode.emit_format(), &options)?
                )
                .map_err(output_error)?;
            } else if args.raw_output && value.is_string() {
                writeln!(output, "{}", value.as_str_content().unwrap_or_default())
                    .map_err(output_error)?;
            } else {
                if !first && !args.no_doc {
                    writeln!(output, "---").map_err(output_error)?;
                }
                write!(
                    output,
                    "{}",
                    emit(&value, output_mode.emit_format(), &options)?
                )
                .map_err(output_error)?;
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
    output_mode: OutputMode,
    context: &RunCtx,
    runtime: &ResolutionRuntime,
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
        let mut records = collect_inputs(std::slice::from_ref(path), args, context, runtime)?;
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
        let file_last = {
            let mut output = BufWriter::new(temporary.as_file_mut());
            let last = run_records(
                filter,
                context,
                records.into_iter(),
                args,
                output_mode,
                &mut output,
            )?
            .ok_or_else(|| {
                YgError::Runtime(format!(
                    "{path}: filter produced no output; refusing to replace the input file"
                ))
            })?;
            output.flush().map_err(|error| {
                YgError::Runtime(format!(
                    "failed to finish temporary output for {path}: {error}"
                ))
            })?;
            last
        };
        temporary.as_file().sync_all().map_err(|error| {
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

    match std::os::unix::fs::chown(temporary, Some(source.uid()), Some(source.gid())) {
        Ok(()) => Ok(()),
        Err(error) => handle_owner_preservation_error(error, path),
    }
}

#[cfg(unix)]
fn handle_owner_preservation_error(
    error: std::io::Error,
    path: &camino::Utf8Path,
) -> Result<(), YgError> {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        eprintln!(
            "warning: could not preserve ownership for {path}: {error}; replacement will be owned by the current user"
        );
        Ok(())
    } else {
        Err(YgError::Runtime(format!(
            "failed to preserve ownership for {path}: {error}"
        )))
    }
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

fn deferred_inputs<'inputs>(
    files: &'inputs [camino::Utf8PathBuf],
    args: &'inputs QueryArgs,
    context: &'inputs RunCtx,
    runtime: &'inputs ResolutionRuntime,
) -> impl Iterator<Item = InputRecord> + 'inputs {
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
                match collect_inputs(files, args, context, runtime) {
                    Ok(inputs) => records.extend(inputs),
                    Err(error) => records.push_back(InputRecord {
                        metadata: InputMeta::default(),
                        value: Err(error),
                        resolution: None,
                    }),
                }
                continue;
            }

            if files.is_empty() {
                if loaded_stdin {
                    return None;
                }
                loaded_stdin = true;
                match collect_inputs(&[], args, context, runtime) {
                    Ok(inputs) => records.extend(inputs),
                    Err(error) => records.push_back(InputRecord {
                        metadata: InputMeta::default(),
                        value: Err(error),
                        resolution: None,
                    }),
                }
                continue;
            }

            let path = files.get(next_file)?;
            let file_index = next_file;
            next_file += 1;
            match collect_inputs(std::slice::from_ref(path), args, context, runtime) {
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
                    resolution: None,
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
    context: &RunCtx,
    runtime: &ResolutionRuntime,
) -> Result<Vec<InputRecord>, YgError> {
    let mut records = Vec::new();
    if files.is_empty() {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| {
                YgError::InputRead(format!("failed to read standard input: {error}"))
            })?;
        append_input(
            &mut records,
            input,
            InputMeta::default(),
            None,
            args,
            context,
            runtime,
        );
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
                InputMeta {
                    filename: (path != "-").then(|| Rc::<str>::from(path.as_str())),
                    file_index,
                    document_index: 0,
                },
                (path != "-").then_some(path.as_path()),
                args,
                context,
                runtime,
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
        let (value, resolution) = if context.config.resolution.is_enabled()
            && (args.base_uri.is_some() || context.config.resolution.files.root.is_some())
        {
            let source = resolution_source(args, &context.config, runtime, None)?;
            register_constructed_value(context, &source, value)?
        } else {
            (value, None)
        };
        return Ok(vec![InputRecord {
            metadata: InputMeta::default(),
            value: Ok(value),
            resolution,
        }]);
    }
    Ok(records)
}

fn append_input(
    records: &mut Vec<InputRecord>,
    input: String,
    metadata: InputMeta,
    source_path: Option<&camino::Utf8Path>,
    args: &QueryArgs,
    context: &RunCtx,
    runtime: &ResolutionRuntime,
) {
    let InputMeta {
        filename,
        file_index,
        ..
    } = metadata;
    let source = if context.config.resolution.is_enabled() {
        match resolution_source(args, &context.config, runtime, source_path) {
            Ok(source) => Some(source),
            Err(error) => {
                records.push(InputRecord {
                    metadata: InputMeta {
                        filename,
                        file_index,
                        document_index: 0,
                    },
                    value: Err(error),
                    resolution: None,
                });
                return;
            }
        }
    } else {
        None
    };

    if args.raw_input {
        let values = if args.slurp {
            vec![YamlVal::str(input)]
        } else {
            input.lines().map(YamlVal::str).collect::<Vec<_>>()
        };
        for (document_index, value) in values.into_iter().enumerate() {
            let registered = match &source {
                Some(source) => register_anonymous_value(context, source, value),
                None => Ok((value, None)),
            };
            match registered {
                Ok((value, resolution)) => records.push(InputRecord {
                    metadata: InputMeta {
                        filename: filename.clone(),
                        file_index,
                        document_index,
                    },
                    value: Ok(value),
                    resolution,
                }),
                Err(error) => records.push(InputRecord {
                    metadata: InputMeta {
                        filename: filename.clone(),
                        file_index,
                        document_index,
                    },
                    value: Err(error.with_input_context(
                        filename.as_deref().unwrap_or("<stdin>"),
                        document_index,
                    )),
                    resolution: None,
                }),
            }
        }
        return;
    }

    if let Some(source) = source {
        match resolve_documents(
            &input,
            source.base,
            Arc::clone(&source.retriever),
            &source.config,
        ) {
            Ok(inputs) => {
                for (document_index, input) in inputs.into_iter().enumerate() {
                    let value = context.register_resolved_input(input);
                    match value {
                        Ok((value, resolution)) => records.push(InputRecord {
                            metadata: InputMeta {
                                filename: filename.clone(),
                                file_index,
                                document_index,
                            },
                            value: Ok(value),
                            resolution: Some(resolution),
                        }),
                        Err(error) => records.push(InputRecord {
                            metadata: InputMeta {
                                filename: filename.clone(),
                                file_index,
                                document_index,
                            },
                            value: Err(error.with_input_context(
                                filename.as_deref().unwrap_or("<stdin>"),
                                document_index,
                            )),
                            resolution: None,
                        }),
                    }
                }
            }
            Err(error) => records.push(InputRecord {
                metadata: InputMeta {
                    filename: filename.clone(),
                    file_index,
                    document_index: 0,
                },
                value: Err(error.with_input_source(filename.as_deref().unwrap_or("<stdin>"))),
                resolution: None,
            }),
        }
        return;
    }

    match yamalgam_query::parse_str_with_config(&input, &context.config) {
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
                        resolution: None,
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
            resolution: None,
        }),
    }
}

// r[impl datamodel.schema.default]
// r[impl datamodel.schema.failsafe]
// r[impl datamodel.schema.json]
// r[impl datamodel.schema.yaml11]
// r[impl resolution.modes]
// r[impl cli.allow-includes]
// r[impl cli.resolve-includes]
// r[impl cli.allow-refs]
// r[impl cli.resolve-refs]
// r[impl cli.allow-ref-host]
// r[impl cli.resolution-root]
// r[impl cli.base-uri]
fn loader_config(args: &QueryArgs) -> Result<LoaderConfig, YgError> {
    let mut config = if args.strict {
        LoaderConfig::strict()
    } else if args.trusted {
        LoaderConfig::trusted()
    } else {
        LoaderConfig::moderate()
    };
    config = config.with_tag_resolution(match args.schema {
        SchemaArg::Core => TagResolution::Yaml12,
        SchemaArg::Failsafe => TagResolution::Failsafe,
        SchemaArg::Json => TagResolution::Json,
        SchemaArg::Yaml11 => TagResolution::Yaml11,
    });

    config.resolution.include.mode = if args.resolve_includes {
        ResolutionMode::Automatic
    } else if args.allow_includes {
        ResolutionMode::Explicit
    } else {
        ResolutionMode::Disabled
    };
    let (ref_mode, schemes) = selected_ref_schemes(args)?;
    config.resolution.refs.mode = ref_mode;
    config.resolution.refs.allow_schemes = schemes;
    config.resolution.refs.allow_hosts = normalized_ref_hosts(args)?;

    let network_enabled = config
        .resolution
        .refs
        .allow_schemes
        .iter()
        .any(|scheme| matches!(scheme.as_str(), "http" | "https"));
    if network_enabled && config.resolution.refs.allow_hosts.is_empty() {
        return Err(YgError::Usage(
            "http or https reference resolution requires --allow-ref-host".into(),
        ));
    }
    if !network_enabled && !config.resolution.refs.allow_hosts.is_empty() {
        return Err(YgError::Usage(
            "--allow-ref-host requires http or https reference resolution".into(),
        ));
    }

    if let Some(root) = &args.resolution_root {
        config.resolution.files.root = Some(canonical_directory(root)?);
    }
    if let Some(value) = &args.base_uri {
        let base = parse_base_uri(value)?;
        validate_base_in_root(&base, config.resolution.files.root.as_deref())?;
    }

    Ok(config)
}

impl ResolutionRuntime {
    fn new(config: &LoaderConfig) -> Result<Self, YgError> {
        let network_enabled = config
            .resolution
            .refs
            .allow_schemes
            .iter()
            .any(|scheme| matches!(scheme.as_str(), "http" | "https"));
        let http = network_enabled
            .then(|| {
                LibrebarRetriever::new(
                    &config.resolution.refs,
                    config.limits.max_input_bytes.unwrap_or(0),
                )
                .map(Arc::new)
                .map_err(|error| {
                    YgError::Runtime(format!("failed to initialize HTTP retrieval: {error}"))
                })
            })
            .transpose()?;
        Ok(Self { http })
    }

    fn router(&self, config: &LoaderConfig) -> Result<Arc<RetrieverRouter>, YgError> {
        let mut router = RetrieverRouter::default();
        if file_resolution_enabled(config) {
            let files = FileRetriever::open(&config.resolution.files).map_err(|error| {
                YgError::Usage(format!(
                    "failed to initialize filesystem resolution: {error}"
                ))
            })?;
            router
                .register("file", files)
                .map_err(internal_router_error)?;
        }
        if let Some(http) = &self.http {
            for scheme in config
                .resolution
                .refs
                .allow_schemes
                .iter()
                .filter(|scheme| matches!(scheme.as_str(), "http" | "https"))
            {
                router
                    .register(scheme, Arc::clone(http))
                    .map_err(internal_router_error)?;
            }
        }
        Ok(Arc::new(router))
    }
}

impl QueryValidationResolver {
    fn new(config: &LoaderConfig, runtime: &ResolutionRuntime) -> Result<Self, YgError> {
        let mut config = config.clone();
        config.resolution.include.mode = ResolutionMode::Disabled;
        let file_enabled = config
            .resolution
            .refs
            .allow_schemes
            .iter()
            .any(|scheme| scheme == "file");
        if file_enabled && config.resolution.files.root.is_none() {
            config.resolution.files.root = Some(std::env::current_dir().map_err(|error| {
                YgError::Runtime(format!(
                    "failed to determine schema resolution root: {error}"
                ))
            })?);
        }
        let root = config.resolution.files.root.clone();
        let retriever: Arc<dyn ResourceRetriever> = runtime.router(&config)?;
        Ok(Self {
            root,
            session: SchemaSession::new(retriever, config),
            validators: Mutex::new(HashMap::new()),
        })
    }

    fn schema_id(&self, schema_ref: &str) -> Result<ResourceId, ValidationFailure> {
        let id = ResourceId::new(schema_ref).map_err(|error| {
            query_validation_failure("schema/invalid-reference", error.to_string())
        })?;
        let Some(path) = id.as_path() else {
            return Ok(id);
        };
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.as_ref().map_or_else(
                || {
                    Err(query_validation_failure(
                        "schema/missing-base",
                        "relative schema reference requires --resolution-root",
                    ))
                },
                |root| Ok(root.join(path)),
            )?
        };
        Ok(ResourceId::from_file(
            std::fs::canonicalize(&path).unwrap_or(path),
        ))
    }

    fn lock_validators(&self) -> MutexGuard<'_, HashMap<ResourceId, Arc<SchemaValidator>>> {
        self.validators
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl QuerySchemaResolver for QueryValidationResolver {
    fn validator(
        &self,
        schema_ref: Option<&str>,
        input: &YamlVal,
    ) -> Result<Arc<SchemaValidator>, ValidationFailure> {
        let schema_ref = schema_ref
            .or_else(|| input.get_str("$schema").and_then(YamlVal::as_str_content))
            .ok_or_else(|| {
                query_validation_failure(
                    "schema/missing",
                    "validate requires a schema argument or top-level $schema property",
                )
            })?;
        let id = self.schema_id(schema_ref)?;
        if let Some(validator) = self.lock_validators().get(&id) {
            return Ok(Arc::clone(validator));
        }

        let document = self
            .session
            .load(&id)
            .map_err(|error| query_validation_failure("schema/retrieval", error.to_string()))?;
        let validator = Arc::new(
            self.session
                .compile_with_options(document, SchemaOptions::default())
                .map_err(|error| query_validation_failure("schema/invalid", error.to_string()))?,
        );
        self.lock_validators().insert(id, Arc::clone(&validator));
        Ok(validator)
    }
}

fn query_validation_failure(
    code: impl Into<String>,
    message: impl Into<String>,
) -> ValidationFailure {
    ValidationFailure {
        report: ValidationReport {
            valid: false,
            diagnostics: vec![ValidationDiagnostic {
                code: code.into(),
                severity: Severity::Error,
                message: message.into(),
                keyword: String::new(),
                instance_path: String::new(),
                schema_path: String::new(),
                expected: None,
                actual: None,
                span: None,
                labels: Vec::new(),
            }],
        },
    }
}

fn internal_router_error(error: RetrieveError) -> YgError {
    YgError::Runtime(format!("failed to configure resource routing: {error}"))
}

fn selected_ref_schemes(args: &QueryArgs) -> Result<(ResolutionMode, Vec<String>), YgError> {
    let (mode, selected) = if !args.resolve_refs.is_empty() {
        (ResolutionMode::Automatic, args.resolve_refs.as_slice())
    } else if !args.allow_refs.is_empty() {
        (ResolutionMode::Explicit, args.allow_refs.as_slice())
    } else {
        (ResolutionMode::Disabled, &[][..])
    };
    let mut unique = BTreeSet::new();
    let mut schemes = Vec::with_capacity(selected.len());
    for scheme in selected.iter().copied().map(RefSchemeArg::as_str) {
        if !unique.insert(scheme) {
            return Err(YgError::Usage(format!(
                "duplicate reference scheme {scheme}"
            )));
        }
        schemes.push(scheme.to_owned());
    }
    Ok((mode, schemes))
}

fn normalized_ref_hosts(args: &QueryArgs) -> Result<Vec<String>, YgError> {
    let mut hosts = BTreeSet::new();
    for candidate in &args.allow_ref_hosts {
        if candidate.is_empty()
            || candidate.trim() != candidate
            || (candidate.contains(':')
                && !(candidate.starts_with('[') && candidate.ends_with(']')))
        {
            return Err(YgError::Usage(format!(
                "invalid --allow-ref-host {candidate:?}: expected a hostname without scheme or port"
            )));
        }
        let rendered = format!("https://{candidate}/");
        let id = ResourceId::new(&rendered).map_err(|error| {
            YgError::Usage(format!("invalid --allow-ref-host {candidate:?}: {error}"))
        })?;
        let url = id.as_url().ok_or_else(|| {
            YgError::Usage(format!(
                "invalid --allow-ref-host {candidate:?}: expected a hostname"
            ))
        })?;
        if id.has_userinfo()
            || url.port().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(YgError::Usage(format!(
                "invalid --allow-ref-host {candidate:?}: expected a hostname without scheme or port"
            )));
        }
        let host = url.host_str().ok_or_else(|| {
            YgError::Usage(format!(
                "invalid --allow-ref-host {candidate:?}: expected a hostname"
            ))
        })?;
        hosts.insert(host.to_ascii_lowercase());
    }
    Ok(hosts.into_iter().collect())
}

fn parse_base_uri(value: &str) -> Result<ResourceId, YgError> {
    if value.contains('#') {
        return Err(YgError::Usage(
            "--base-uri must not contain a fragment".into(),
        ));
    }
    let base = ResourceId::new(value)
        .map_err(|error| YgError::Usage(format!("invalid --base-uri {value:?}: {error}")))?;
    if let Some(path) = base.as_path() {
        return if path.is_absolute() {
            Ok(base)
        } else {
            Err(YgError::Usage(
                "--base-uri must be an absolute URI or absolute file path".into(),
            ))
        };
    }
    if base.as_url().is_none() {
        return Err(YgError::Usage(
            "unsupported --base-uri resource kind".into(),
        ));
    }
    if !matches!(base.scheme(), "http" | "https") {
        return Err(YgError::Usage(format!(
            "unsupported --base-uri scheme {}; expected file, http, or https",
            base.scheme()
        )));
    }
    if base.has_userinfo() || base.host_str().is_none() {
        return Err(YgError::Usage(
            "http and https --base-uri values require a host and no userinfo".into(),
        ));
    }
    Ok(base)
}

fn canonical_directory(path: &camino::Utf8Path) -> Result<PathBuf, YgError> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| YgError::Usage(format!("invalid --resolution-root {path}: {error}")))?;
    if !canonical.is_dir() {
        return Err(YgError::Usage(format!(
            "invalid --resolution-root {path}: not a directory"
        )));
    }
    Ok(canonical)
}

fn canonical_file_base(path: &Path) -> Result<PathBuf, YgError> {
    if path.exists() {
        let canonical = std::fs::canonicalize(path).map_err(|error| {
            YgError::Usage(format!("invalid file base {}: {error}", path.display()))
        })?;
        if canonical.is_dir() {
            return Err(YgError::Usage(format!(
                "invalid file base {}: expected a document path",
                path.display()
            )));
        }
        return Ok(canonical);
    }
    let name = path.file_name().ok_or_else(|| {
        YgError::Usage(format!(
            "invalid file base {}: expected a document path",
            path.display()
        ))
    })?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = std::fs::canonicalize(parent).map_err(|error| {
        YgError::Usage(format!(
            "invalid file base {}: containing directory is unavailable: {error}",
            path.display()
        ))
    })?;
    Ok(parent.join(name))
}

fn canonical_base(base: ResourceId) -> Result<ResourceId, YgError> {
    base.as_path().map_or_else(
        || Ok(base.clone()),
        |path| canonical_file_base(path).map(ResourceId::from_file),
    )
}

fn validate_base_in_root(base: &ResourceId, root: Option<&Path>) -> Result<(), YgError> {
    let (Some(path), Some(root)) = (base.as_path(), root) else {
        return Ok(());
    };
    let canonical = canonical_file_base(path)?;
    if !canonical.starts_with(root) {
        return Err(YgError::Usage(format!(
            "file base {} is outside --resolution-root {}",
            canonical.display(),
            root.display()
        )));
    }
    Ok(())
}

fn file_resolution_enabled(config: &LoaderConfig) -> bool {
    config.resolution.include.mode.allows_explicit()
        || (config.resolution.refs.mode.allows_explicit()
            && config
                .resolution
                .refs
                .allow_schemes
                .iter()
                .any(|scheme| scheme == "file"))
}

fn resolution_source(
    args: &QueryArgs,
    global: &LoaderConfig,
    runtime: &ResolutionRuntime,
    source_path: Option<&camino::Utf8Path>,
) -> Result<ResolutionSource, YgError> {
    let mut config = global.clone();
    let base = if let Some(path) = source_path {
        let canonical = std::fs::canonicalize(path).map_err(|error| {
            YgError::InputRead(format!("failed to resolve input path {path}: {error}"))
        })?;
        ResourceId::from_file(canonical)
    } else if let Some(value) = &args.base_uri {
        canonical_base(parse_base_uri(value)?)?
    } else if let Some(root) = &config.resolution.files.root {
        ResourceId::from_file(root.join(".yg-stdin.yaml"))
    } else {
        ResourceId::anonymous()
    };

    if source_path.is_none()
        && config.resolution.include.mode.allows_explicit()
        && config.resolution.files.root.is_none()
    {
        return Err(YgError::Usage(
            "standard input includes require --resolution-root".into(),
        ));
    }
    if config.resolution.include.mode.allows_explicit() && base.as_path().is_none() {
        return Err(YgError::Usage(
            "standard input includes require a file base under --resolution-root".into(),
        ));
    }

    if config.resolution.files.root.is_none()
        && let Some(path) = base.as_path()
    {
        config.resolution.files.root = path.parent().map(Path::to_path_buf);
    }
    if file_resolution_enabled(&config) && config.resolution.files.root.is_none() {
        return Err(YgError::Usage(
            "relative filesystem access from standard input requires --base-uri or --resolution-root"
                .into(),
        ));
    }
    validate_base_in_root(&base, config.resolution.files.root.as_deref())?;
    let retriever = runtime.router(&config)?;
    Ok(ResolutionSource {
        base,
        config,
        retriever,
    })
}

fn register_anonymous_value(
    context: &RunCtx,
    source: &ResolutionSource,
    value: YamlVal,
) -> Result<(YamlVal, Option<ResolutionSessionKey>), YgError> {
    let input = ResolvedInput {
        value,
        resolution: resolution_handle(
            Arc::clone(&source.retriever),
            Some(source.base.clone()),
            source.config.clone(),
        ),
    };
    let (mut value, key) = context.register_resolved_input(input)?;
    let mut metadata = value.meta().cloned().unwrap_or_else(Meta::default);
    metadata.origin = Some(ResourceOrigin {
        resource: source.base.clone(),
        session: key,
    });
    value = value.with_meta(metadata);
    Ok((value, Some(key)))
}

fn register_constructed_value(
    context: &RunCtx,
    source: &ResolutionSource,
    value: YamlVal,
) -> Result<(YamlVal, Option<ResolutionSessionKey>), YgError> {
    let resolution = resolution_handle(
        Arc::clone(&source.retriever),
        Some(source.base.clone()),
        source.config.clone(),
    );
    let key = context.register_resolution(resolution)?;
    let mut metadata = value.meta().cloned().unwrap_or_else(Meta::default);
    metadata.origin = Some(ResourceOrigin {
        resource: source.base.clone(),
        session: key,
    });
    Ok((value.with_meta(metadata), Some(key)))
}

fn anonymous_resolution(
    args: &QueryArgs,
    context: &RunCtx,
    runtime: &ResolutionRuntime,
) -> Result<Option<ResolutionSessionKey>, YgError> {
    if !context.config.resolution.is_enabled() {
        return Ok(None);
    }
    let source = resolution_source(args, &context.config, runtime, None)?;
    let (_, key) = register_anonymous_value(context, &source, YamlVal::null())?;
    Ok(key)
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
        YgError::InputParse(_)
        | YgError::InputLimit { .. }
        | YgError::InputConversion { .. }
        | YgError::Runtime(_)
        | YgError::RuntimeLimit { .. }
        | YgError::ResourceOverflow { .. } => ExitCode::from(5),
        YgError::Halt(code) => ExitCode::from(halt_exit_code(*code)),
        YgError::BrokenPipe => ExitCode::SUCCESS,
        _ => ExitCode::from(5),
    }
}

fn halt_exit_code(code: i32) -> u8 {
    // Shell status is one byte. Preserve valid jq/POSIX values, but make every
    // out-of-range request a generic failure instead of modulo-wrapping a
    // nonzero halt code such as 256 to success.
    u8::try_from(code).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn ownership_permission_denial_warns_but_other_errors_remain_fatal() {
        let path = camino::Utf8Path::new("config.yml");

        assert!(
            handle_owner_preservation_error(
                std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                path,
            )
            .is_ok()
        );
        assert!(matches!(
            handle_owner_preservation_error(std::io::Error::other("device failure"), path),
            Err(YgError::Runtime(message)) if message.contains("device failure")
        ));
    }

    #[test]
    fn resolution_policy_normalizes_cli_schemes_and_hosts_once() {
        let args = QueryArgs {
            resolve_includes: true,
            allow_refs: vec![RefSchemeArg::Https, RefSchemeArg::Http],
            allow_ref_hosts: vec!["EXAMPLE.Test".into(), "example.test".into()],
            ..QueryArgs::default()
        };

        let config = loader_config(&args).unwrap();

        assert_eq!(config.resolution.include.mode, ResolutionMode::Automatic);
        assert_eq!(config.resolution.refs.mode, ResolutionMode::Explicit);
        assert_eq!(config.resolution.refs.allow_schemes, ["https", "http"]);
        assert_eq!(config.resolution.refs.allow_hosts, ["example.test"]);
    }
}
