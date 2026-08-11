//! JSON Schema validation for YAML document streams.

use std::collections::{BTreeSet, HashMap};
use std::io::{BufWriter, Read as _, Write as _};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, anyhow, bail};
use camino::Utf8PathBuf;
use clap::Args;
use serde::Serialize;
use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_query::{EmitFormat, EmitOpts, YamlVal};
use yamalgam_resolve::{
    FileRetriever, LibrebarRetriever, ResourceId, ResourceRetriever, RetrieverRouter,
};
use yamalgam_schema::{
    SchemaCatalog, SchemaOptions, SchemaSelector, SchemaSession, SchemaValidator,
    ValidationDiagnostic,
};

use crate::RefSchemeArg;
use crate::config::Config;

/// Arguments for `yg validate` and its `lint` alias.
#[derive(Args, Clone, Debug)]
pub struct ValidateArgs {
    /// Input files (standard input when omitted).
    #[arg(value_name = "FILE")]
    pub files: Vec<Utf8PathBuf>,

    /// Schema URI applied to every input document.
    #[arg(long, value_name = "URI")]
    pub schema: Option<String>,

    /// Enable the configured SchemaStore-compatible catalog.
    #[arg(long)]
    pub schema_store: bool,

    /// Maximum validation diagnostics retained per document.
    #[arg(long, default_value_t = 100)]
    pub max_diagnostics: usize,

    /// Permit schema retrieval through SCHEME in addition to local files.
    #[arg(long, value_delimiter = ',', value_enum, ignore_case = true)]
    pub allow_refs: Vec<RefSchemeArg>,

    /// Permit HTTP schema retrieval from exactly this host (repeatable).
    #[arg(long = "allow-ref-host")]
    pub allow_ref_hosts: Vec<String>,

    /// Confine local schema and catalog retrieval to this directory.
    #[arg(long, value_name = "DIR")]
    pub resolution_root: Option<Utf8PathBuf>,
}

#[derive(Serialize)]
struct DocumentReport {
    filename: String,
    document: usize,
    valid: bool,
    diagnostics: Vec<ValidationDiagnostic>,
}

struct InputStream {
    filename: String,
    containing: Option<ResourceId>,
    source: String,
}

/// Validate every input document and write one deterministic report array.
pub fn cmd_validate(
    args: &ValidateArgs,
    json_output: bool,
    config: &Config,
) -> anyhow::Result<ExitCode> {
    let loader = match loader_config(args) {
        Ok(loader) => loader,
        Err(error) => {
            eprintln!("yg: {error:#}");
            return Ok(ExitCode::from(2));
        }
    };
    let mut instance_loader = loader.clone();
    instance_loader.resolution.refs.mode = ResolutionMode::Disabled;
    let retriever = retriever(&loader)?;
    let session = SchemaSession::new(Arc::clone(&retriever), loader.clone());
    let schema_store = if args.schema_store || config.schema_store.enabled {
        Some(load_catalog(&session, &config.schema_store.url)?)
    } else {
        None
    };
    let selector = SchemaSelector {
        associations: config.schemas.clone(),
        schema_store,
    };
    let mut validators = HashMap::<ResourceId, Arc<SchemaValidator>>::new();
    let mut reports = Vec::new();

    for input in read_inputs(args)? {
        let documents =
            yamalgam_compose::from_str_located_with_config(&input.source, &instance_loader)
                .map_err(|error| anyhow!("invalid YAML in {}: {error}", input.filename))?;
        let selected = selector
            .select(
                &input.source,
                input.containing.as_ref(),
                args.schema.as_deref(),
            )
            .with_context(|| format!("failed to select a schema for {}", input.filename))?;
        if documents.len() != selected.len() {
            bail!(
                "schema selection returned {} documents for {} composed documents",
                selected.len(),
                documents.len()
            );
        }

        for (document, (instance, selection)) in documents.into_iter().zip(selected).enumerate() {
            let selection = selection.ok_or_else(|| {
                anyhow!("{} document {document}: no schema selected", input.filename)
            })?;
            let id = canonical_resource(ResourceId::new(&selection.uri).with_context(|| {
                format!(
                    "invalid selected schema URI for {} document {document}",
                    input.filename
                )
            })?)?;
            let validator = match validators.get(&id) {
                Some(validator) => Arc::clone(validator),
                None => {
                    let root = session.load(&id)?;
                    let validator = Arc::new(session.compile_with_options(
                        root,
                        SchemaOptions {
                            max_diagnostics: args.max_diagnostics,
                            ..SchemaOptions::default()
                        },
                    )?);
                    validators.insert(id, Arc::clone(&validator));
                    validator
                }
            };
            let report = validator.validate_located(&instance);
            reports.push(DocumentReport {
                filename: input.filename.clone(),
                document,
                valid: report.valid,
                diagnostics: report.diagnostics,
            });
        }
    }

    write_reports(&reports, json_output, &loader)?;
    Ok(if reports.iter().all(|report| report.valid) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn loader_config(args: &ValidateArgs) -> anyhow::Result<LoaderConfig> {
    let mut config = LoaderConfig::moderate();
    config.resolution.refs.mode = ResolutionMode::Explicit;
    let mut schemes = BTreeSet::from(["file".to_owned()]);
    schemes.extend(
        args.allow_refs
            .iter()
            .copied()
            .map(RefSchemeArg::as_str)
            .map(str::to_owned),
    );
    config.resolution.refs.allow_schemes = schemes.into_iter().collect();
    config.resolution.refs.allow_hosts = normalized_hosts(&args.allow_ref_hosts)?;

    let network_enabled = config
        .resolution
        .refs
        .allow_schemes
        .iter()
        .any(|scheme| matches!(scheme.as_str(), "http" | "https"));
    if network_enabled && config.resolution.refs.allow_hosts.is_empty() {
        bail!("http or https schema retrieval requires --allow-ref-host");
    }
    if !network_enabled && !config.resolution.refs.allow_hosts.is_empty() {
        bail!("--allow-ref-host requires --allow-refs http or https");
    }

    let root = args.resolution_root.as_deref().map_or_else(
        || std::env::current_dir().context("failed to determine the schema resolution root"),
        |root| {
            std::fs::canonicalize(root).with_context(|| format!("invalid --resolution-root {root}"))
        },
    )?;
    if !root.is_dir() {
        bail!(
            "schema resolution root is not a directory: {}",
            root.display()
        );
    }
    config.resolution.files.root = Some(root);
    Ok(config)
}

fn normalized_hosts(hosts: &[String]) -> anyhow::Result<Vec<String>> {
    let mut normalized = BTreeSet::new();
    for host in hosts {
        if host.is_empty()
            || host.trim() != host
            || (host.contains(':') && !(host.starts_with('[') && host.ends_with(']')))
        {
            bail!("invalid --allow-ref-host {host:?}: expected a hostname without scheme or port");
        }
        let candidate = ResourceId::new(&format!("https://{host}/"))
            .with_context(|| format!("invalid --allow-ref-host {host:?}"))?;
        let url = candidate
            .as_url()
            .ok_or_else(|| anyhow!("invalid --allow-ref-host {host:?}"))?;
        if candidate.has_userinfo()
            || url.port().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("invalid --allow-ref-host {host:?}: expected a hostname without scheme or port");
        }
        normalized.insert(
            url.host_str()
                .ok_or_else(|| anyhow!("invalid --allow-ref-host {host:?}"))?
                .to_ascii_lowercase(),
        );
    }
    Ok(normalized.into_iter().collect())
}

fn retriever(config: &LoaderConfig) -> anyhow::Result<Arc<dyn ResourceRetriever>> {
    let mut router = RetrieverRouter::default();
    router.register("file", FileRetriever::open(&config.resolution.files)?)?;

    let network_schemes = config
        .resolution
        .refs
        .allow_schemes
        .iter()
        .filter(|scheme| matches!(scheme.as_str(), "http" | "https"))
        .cloned()
        .collect::<Vec<_>>();
    if !network_schemes.is_empty() {
        let http = Arc::new(LibrebarRetriever::new(
            &config.resolution.refs,
            config.limits.max_input_bytes.unwrap_or(0),
        )?);
        for scheme in network_schemes {
            router.register(scheme, Arc::clone(&http))?;
        }
    }
    Ok(Arc::new(router))
}

fn load_catalog(session: &SchemaSession, uri: &str) -> anyhow::Result<SchemaCatalog> {
    let id = canonical_resource(
        ResourceId::new(uri).with_context(|| format!("invalid SchemaStore catalog URI {uri:?}"))?,
    )?;
    let document = session.load(&id)?;
    SchemaCatalog::from_value(&document.document.value).map_err(Into::into)
}

fn read_inputs(args: &ValidateArgs) -> anyhow::Result<Vec<InputStream>> {
    if args.files.is_empty() {
        return read_stdin().map(|source| {
            vec![InputStream {
                filename: "<stdin>".to_owned(),
                containing: None,
                source,
            }]
        });
    }

    let mut used_stdin = false;
    let mut inputs = Vec::with_capacity(args.files.len());
    for path in &args.files {
        if path == "-" {
            if used_stdin {
                bail!("standard input may be listed only once");
            }
            used_stdin = true;
            inputs.push(InputStream {
                filename: "<stdin>".to_owned(),
                containing: None,
                source: read_stdin()?,
            });
            continue;
        }
        let canonical =
            std::fs::canonicalize(path).with_context(|| format!("failed to read input {path}"))?;
        let source = std::fs::read_to_string(&canonical)
            .with_context(|| format!("failed to read input {path}"))?;
        inputs.push(InputStream {
            filename: path.to_string(),
            containing: Some(ResourceId::from_file(canonical)),
            source,
        });
    }
    Ok(inputs)
}

fn read_stdin() -> anyhow::Result<String> {
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .context("failed to read standard input")?;
    Ok(source)
}

fn canonical_resource(id: ResourceId) -> anyhow::Result<ResourceId> {
    let Some(path) = id.as_path() else {
        return Ok(id);
    };
    match std::fs::canonicalize(path) {
        Ok(path) => Ok(ResourceId::from_file(path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(id),
        Err(error) => Err(error).with_context(|| format!("failed to resolve schema resource {id}")),
    }
}

fn write_reports(
    reports: &[DocumentReport],
    json_output: bool,
    loader: &LoaderConfig,
) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    if json_output {
        serde_json::to_writer_pretty(&mut output, reports)?;
        writeln!(output)?;
    } else {
        let json = serde_json::to_value(reports)?;
        let value = json_to_yaml(json)?;
        let mut options = EmitOpts::default();
        options.limits = loader.limits.clone();
        write!(
            output,
            "{}",
            yamalgam_query::emit::emit(&value, EmitFormat::Yaml, &options)?
        )?;
    }
    output.flush()?;
    Ok(())
}

fn json_to_yaml(value: serde_json::Value) -> anyhow::Result<YamlVal> {
    Ok(match value {
        serde_json::Value::Null => YamlVal::null(),
        serde_json::Value::Bool(value) => YamlVal::from(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                YamlVal::int(value)
            } else if let Some(value) = value.as_u64() {
                YamlVal::int(i64::try_from(value).context("report integer exceeds i64")?)
            } else {
                YamlVal::float(
                    value
                        .as_f64()
                        .ok_or_else(|| anyhow!("report number is not finite"))?,
                )
            }
        }
        serde_json::Value::String(value) => YamlVal::from(value),
        serde_json::Value::Array(values) => YamlVal::seq(
            values
                .into_iter()
                .map(json_to_yaml)
                .collect::<anyhow::Result<Vec<_>>>()?,
        ),
        serde_json::Value::Object(entries) => YamlVal::map(
            entries
                .into_iter()
                .map(|(key, value)| Ok((YamlVal::from(key), json_to_yaml(value)?)))
                .collect::<anyhow::Result<Vec<_>>>()?,
        ),
    })
}
