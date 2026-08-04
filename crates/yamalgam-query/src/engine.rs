//! jaq filter compilation and execution.

use std::cell::RefCell;
use std::rc::Rc;

use jaq_core::data::HasLut;
use jaq_core::load::{self, Arena, File, Loader};
use jaq_core::{Ctx, DataT, Filter, Lut, Vars, compile};
use jaq_std::input::{HasInputs, Inputs, RcIter};
use yamalgam_core::{LoaderConfig, TagResolution};

use crate::error::YgError;
use crate::value::YamlVal;

/// jaq data-kind marker for YAML values.
pub struct YgKind;

/// A query filter compiled for yg's YAML value model.
pub type CompiledFilter = Filter<YgKind>;

/// Native capabilities available while compiling a filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompileOptions {
    /// Permit the jq-compatible `env` filter to read process environment variables.
    pub allow_env: bool,
}

impl CompileOptions {
    /// Restrict filters to deterministic capabilities without environment access.
    #[must_use]
    pub const fn sandboxed() -> Self {
        Self { allow_env: false }
    }
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self { allow_env: true }
    }
}

impl DataT for YgKind {
    type V<'a> = YamlVal;
    type Data<'a> = &'a YgData<'a>;
}

/// Global data available to native filters during one run.
pub struct YgData<'a> {
    /// Compiled filter lookup table.
    pub lut: &'a Lut<YgKind>,
    /// Remaining input values for `input` and `inputs`.
    pub inputs: Inputs<'a, YamlVal>,
    /// yg-specific run context.
    pub context: &'a RunCtx,
}

impl<'a> HasLut<'a, YgKind> for &'a YgData<'a> {
    fn lut(&self) -> &'a Lut<YgKind> {
        self.lut
    }
}

impl<'a> HasInputs<'a, YamlVal> for &'a YgData<'a> {
    fn inputs(&self) -> Inputs<'a, YamlVal> {
        self.inputs
    }
}

/// Metadata for the input document currently being evaluated.
#[derive(Clone, Debug, Default)]
pub struct InputMeta {
    /// Input filename, or `None` for standard input.
    pub filename: Option<Rc<str>>,
    /// Zero-based input-file index.
    pub file_index: usize,
    /// Zero-based document index within the file.
    pub document_index: usize,
}

/// Run-wide configuration and current input metadata.
pub struct RunCtx {
    /// YAML loading and schema configuration.
    pub config: LoaderConfig,
    /// Metadata for the document whose filter is currently running.
    input_meta: RefCell<InputMeta>,
    /// Metadata associated with the most recently pulled input value.
    pending_input_meta: RefCell<InputMeta>,
    variables: Vec<YamlVal>,
}

impl RunCtx {
    /// Construct a run context.
    #[must_use]
    pub fn new(config: LoaderConfig) -> Self {
        Self {
            config,
            input_meta: RefCell::new(InputMeta::default()),
            pending_input_meta: RefCell::new(InputMeta::default()),
            variables: Vec::new(),
        }
    }

    /// Attach values for the globals declared while compiling the filter.
    #[must_use]
    pub fn with_variables(mut self, variables: impl IntoIterator<Item = YamlVal>) -> Self {
        self.variables = variables.into_iter().collect();
        self
    }

    /// Stage metadata for the next input value yielded to the engine.
    pub fn stage_input_meta(&self, metadata: InputMeta) {
        *self.pending_input_meta.borrow_mut() = metadata;
    }

    pub(crate) fn input_meta(&self) -> std::cell::Ref<'_, InputMeta> {
        self.input_meta.borrow()
    }

    fn activate_pending_input_meta(&self) {
        *self.input_meta.borrow_mut() = self.pending_input_meta.borrow().clone();
    }

    /// Return the active schema's CLI spelling.
    #[must_use]
    pub const fn schema_name(&self) -> &'static str {
        match self.config.tag_resolution {
            TagResolution::Yaml12 => "core",
            TagResolution::Failsafe => "failsafe",
            TagResolution::Json => "json",
            TagResolution::Yaml11 => "1.1",
        }
    }
}

// r[impl diverge.keys]
// r[impl diverge.keys-sorted]
// r[impl diverge.keys.seq]
fn yg_defs() -> impl Iterator<Item = load::parse::Def<&'static str>> {
    load::parse(include_str!("defs.jq"), |parser| parser.defs())
        .expect("bundled yg definitions must parse")
        .into_iter()
}

/// Compile a query filter with the yg definitions and native functions.
// r[impl overview.jaq-foundation]
// r[impl filters.identity]
// r[impl filters.pipe]
// r[impl filters.comma]
// r[impl filters.paren]
// r[impl filters.cond.if]
// r[impl filters.cond.and-or-not]
// r[impl filters.cond.try]
// r[impl filters.cond.alternative]
// r[impl filters.binding]
// r[impl filters.reduce]
// r[impl filters.foreach]
// r[impl filters.def]
// r[impl filters.def.recursion]
// r[impl filters.string-interp]
// r[impl phases.core]
pub fn compile_filter(code: &str, variables: &[String]) -> Result<CompiledFilter, YgError> {
    compile_filter_with_options(code, variables, CompileOptions::default())
}

/// Compile a query filter with an explicit native-capability policy.
pub fn compile_filter_with_options(
    code: &str,
    variables: &[String],
    options: CompileOptions,
) -> Result<CompiledFilter, YgError> {
    let arena = Arena::default();
    let definitions = jaq_core::defs().chain(jaq_std::defs()).chain(yg_defs());
    let loader = Loader::new(definitions);
    let modules = loader
        .load(
            &arena,
            File {
                path: "<filter>",
                code,
            },
        )
        .map_err(|errors| YgError::FilterSyntax(render_load_errors(errors)))?;

    let variables = variables
        .iter()
        .map(|variable| format!("${variable}"))
        .collect::<Vec<_>>();
    compile::Compiler::default()
        .with_funs(crate::funs::all_funs(options.allow_env))
        .with_global_vars(variables.iter().map(String::as_str))
        .compile(modules)
        .map_err(|errors| YgError::FilterCompile(render_compile_errors(errors)))
}

/// Run a compiled filter over input values and visit every output.
///
/// The returned boolean is the truthiness of the last output, or `None` when
/// the filter produced no output.
pub fn run_filter<'a, E, IE>(
    filter: &Filter<YgKind>,
    context: &RunCtx,
    inputs: impl Iterator<Item = Result<YamlVal, IE>> + 'a,
    visit: impl FnMut(YamlVal) -> Result<(), E>,
) -> Result<Option<bool>, E>
where
    E: From<YgError>,
    IE: Into<YgError> + 'a,
{
    run_filter_with_null_input(filter, context, inputs, false, visit)
}

/// Run a filter with the real input stream available to `input`/`inputs`.
///
/// When `null_input` is true, the filter driver is a single null while native
/// input filters continue to consume from `inputs`.
pub fn run_filter_with_null_input<'a, E, IE>(
    filter: &Filter<YgKind>,
    context: &RunCtx,
    inputs: impl Iterator<Item = Result<YamlVal, IE>> + 'a,
    null_input: bool,
    mut visit: impl FnMut(YamlVal) -> Result<(), E>,
) -> Result<Option<bool>, E>
where
    E: From<YgError>,
    IE: Into<YgError> + 'a,
{
    let _limit_guard = crate::runtime_limits::activate(&context.config.limits);
    let inputs = inputs.map(|result| result.map_err(|error| error.into().to_string()));
    let inputs: Box<dyn Iterator<Item = Result<YamlVal, String>> + 'a> = Box::new(inputs);
    let inputs = RcIter::new(inputs);
    let data = YgData {
        lut: &filter.lut,
        inputs: &inputs,
        context,
    };
    let jaq_context = Ctx::<YgKind>::new(&data, Vars::new(context.variables.iter().cloned()));
    let mut last = None;

    let mut driver: Box<dyn Iterator<Item = Result<YamlVal, String>> + '_> = if null_input {
        Box::new(core::iter::once(Ok(YamlVal::null())))
    } else {
        Box::new(&inputs)
    };

    for input in &mut driver {
        let input = input.map_err(|error| E::from(YgError::InputParse(error)))?;
        context.activate_pending_input_meta();
        for output in filter.id.run((jaq_context.clone(), input)) {
            let output: YamlVal = output.map_err(|error| {
                E::from(contextualize_runtime_error(
                    context,
                    YgError::runtime(error),
                ))
            })?;
            if let Some(message) = crate::runtime_limits::take_error() {
                return Err(E::from(contextualize_runtime_error(
                    context,
                    YgError::Runtime(message),
                )));
            }
            last = Some(jaq_core::ValT::as_bool(&output));
            visit(output)?;
        }
        if let Some(message) = crate::runtime_limits::take_error() {
            return Err(E::from(contextualize_runtime_error(
                context,
                YgError::Runtime(message),
            )));
        }
    }
    Ok(last)
}

fn contextualize_runtime_error(context: &RunCtx, error: YgError) -> YgError {
    match error {
        YgError::Runtime(message) => {
            let metadata = context.input_meta.borrow();
            let source = metadata
                .filename
                .as_deref()
                .map_or_else(|| "<stdin>".to_owned(), str::to_owned);
            YgError::Runtime(format!(
                "{source} document {}: {message}",
                metadata.document_index + 1
            ))
        }
        error => error,
    }
}

fn render_load_errors(errors: load::Errors<&str, &str>) -> String {
    errors
        .into_iter()
        .flat_map(|(file, error)| match error {
            load::Error::Io(errors) => errors
                .into_iter()
                .map(|(path, error)| format!("{}: could not load {path}: {error}", file.path))
                .collect::<Vec<_>>(),
            load::Error::Lex(errors) => errors
                .into_iter()
                .map(|(expected, found)| {
                    format!(
                        "{}: expected {}, found {found:?}",
                        file.path,
                        expected.as_str()
                    )
                })
                .collect::<Vec<_>>(),
            load::Error::Parse(errors) => errors
                .into_iter()
                .map(|(expected, found)| {
                    format!(
                        "{}: expected {}, found {found:?}",
                        file.path,
                        expected.as_str()
                    )
                })
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_compile_errors(errors: compile::Errors<&str, &str>) -> String {
    errors
        .into_iter()
        .flat_map(|(file, errors)| {
            errors.into_iter().map(move |(found, undefined)| {
                format!("{}: undefined {} {found}", file.path, undefined.as_str())
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}
