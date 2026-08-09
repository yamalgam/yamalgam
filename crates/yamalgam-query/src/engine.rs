//! jaq filter compilation and execution.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use jaq_core::data::HasLut;
use jaq_core::load::{self, Arena, File, Loader};
use jaq_core::{Ctx, DataT, Filter, Lut, Vars, compile};
use jaq_std::input::{HasInputs, Inputs, RcIter};
use yamalgam_core::{LoaderConfig, ResolutionCapability, TagResolution};
use yamalgam_parser::{ResolutionErrorCategory, ResolveError};

use crate::error::YgError;
use crate::resolution::{ResolutionHandle, ResolutionSessionKey, ResolvedInput};
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
    input_resolution: Cell<Option<ResolutionSessionKey>>,
    pending_input_resolution: Cell<Option<ResolutionSessionKey>>,
    variables: Vec<YamlVal>,
    resolutions: RefCell<HashMap<ResolutionSessionKey, ResolutionHandle>>,
    next_resolution_key: Cell<u64>,
}

impl RunCtx {
    /// Construct a run context.
    #[must_use]
    pub fn new(config: LoaderConfig) -> Self {
        Self {
            config,
            input_meta: RefCell::new(InputMeta::default()),
            pending_input_meta: RefCell::new(InputMeta::default()),
            input_resolution: Cell::new(None),
            pending_input_resolution: Cell::new(None),
            variables: Vec::new(),
            resolutions: RefCell::new(HashMap::new()),
            next_resolution_key: Cell::new(1),
        }
    }

    /// Attach values for the globals declared while compiling the filter.
    #[must_use]
    pub fn with_variables(mut self, variables: impl IntoIterator<Item = YamlVal>) -> Self {
        self.variables = variables.into_iter().collect::<Vec<_>>();
        self
    }

    /// Stage metadata for the next input value yielded to the engine.
    pub fn stage_input_meta(&self, metadata: InputMeta) {
        *self.pending_input_meta.borrow_mut() = metadata;
        self.pending_input_resolution.set(None);
    }

    /// Stage the resolution session for the next input value.
    pub fn stage_input_resolution(&self, resolution: Option<ResolutionSessionKey>) {
        self.pending_input_resolution.set(resolution);
    }

    /// Register a loaded value and its retained resolution session.
    ///
    /// Pass the returned key to [`Self::stage_input_resolution`] when this
    /// value is staged for evaluation.
    ///
    /// # Errors
    ///
    /// Returns [`YgError::ResourceOverflow`] if the run-local key space is
    /// exhausted. Keys are never reused.
    pub fn register_resolved_input(
        &self,
        mut input: ResolvedInput,
    ) -> Result<(YamlVal, ResolutionSessionKey), YgError> {
        let raw = self.next_resolution_key.get();
        let next = raw.checked_add(1).ok_or_else(|| {
            YgError::resource_overflow("resolution session key", "resolution session key overflow")
        })?;
        let key = ResolutionSessionKey(raw);
        crate::resolution::assign_session(&mut input.value, key);
        self.resolutions.borrow_mut().insert(key, input.resolution);
        self.next_resolution_key.set(next);
        Ok((input.value, key))
    }

    /// Return a clone of a registered resolution handle.
    #[must_use]
    pub fn resolution_handle(&self, key: ResolutionSessionKey) -> Option<ResolutionHandle> {
        self.resolutions.borrow().get(&key).cloned()
    }

    #[allow(clippy::result_large_err)] // QueryResolver returns the shared typed error directly.
    pub(crate) fn resolve_value(
        &self,
        capability: ResolutionCapability,
        value: YamlVal,
    ) -> Result<YamlVal, ResolveError> {
        if !self.config.resolution.allows_explicit(capability) {
            return Err(crate::runtime_resolution::resolution_error(
                ResolutionErrorCategory::DisabledCapability,
                capability,
                "<query value>",
            ));
        }
        let value_session = value
            .meta()
            .and_then(|metadata| metadata.origin.as_ref())
            .map(|origin| origin.session)
            .filter(|key| *key != ResolutionSessionKey::UNREGISTERED);
        let active_session = self.input_resolution.get();
        let key = value_session.or(active_session).ok_or_else(|| {
            crate::runtime_resolution::resolution_error(
                ResolutionErrorCategory::MissingBase,
                capability,
                "<query value>",
            )
        })?;
        let handle = self.resolution_handle(key).ok_or_else(|| {
            crate::runtime_resolution::resolution_error(
                ResolutionErrorCategory::InvalidResource,
                capability,
                "<unknown query session>",
            )
        })?;
        let mut resolved = handle.borrow_mut().resolve_value(capability, value)?;
        crate::resolution::assign_session(&mut resolved, key);
        Ok(resolved)
    }

    pub(crate) fn input_meta(&self) -> std::cell::Ref<'_, InputMeta> {
        self.input_meta.borrow()
    }

    fn activate_pending_input_meta(&self) {
        *self.input_meta.borrow_mut() = self.pending_input_meta.borrow().clone();
        self.input_resolution
            .set(self.pending_input_resolution.get());
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
    let code = alias_include_filter(code);
    let modules = loader
        .load(
            &arena,
            File {
                path: "<filter>",
                code: code.as_ref(),
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

fn alias_include_filter(code: &str) -> Cow<'_, str> {
    const FILTER: &str = "include";
    const ALIAS: &str = "yginc__";
    let bytes = code.as_bytes();
    let mut output = None::<String>;
    let mut index = 0;
    let mut string = false;
    let mut escaped = false;
    let mut comment = false;
    let mut interpolations = Vec::<usize>::new();

    while index < bytes.len() {
        let byte = bytes[index];
        if comment {
            if byte == b'\n' {
                comment = false;
            }
            index += 1;
            continue;
        }
        if string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' && bytes.get(index + 1) == Some(&b'(') {
                interpolations.push(1);
                string = false;
                index += 2;
                continue;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'#' {
            comment = true;
            index += 1;
            continue;
        }
        if byte == b'"' {
            string = true;
            index += 1;
            continue;
        }
        if let Some(depth) = interpolations.last_mut() {
            if byte == b'(' {
                *depth = depth.saturating_add(1);
                index += 1;
                continue;
            }
            if byte == b')' {
                if *depth == 1 {
                    interpolations.pop();
                    string = true;
                } else {
                    *depth -= 1;
                }
                index += 1;
                continue;
            }
        }
        if bytes[index..].starts_with(FILTER.as_bytes())
            && is_identifier_boundary(bytes, index.wrapping_sub(1))
            && is_identifier_boundary(bytes, index + FILTER.len())
            && is_filter_position(bytes, index, index + FILTER.len())
        {
            let rewritten = output.get_or_insert_with(|| code.to_owned());
            rewritten.replace_range(index..index + FILTER.len(), ALIAS);
            index += ALIAS.len();
            continue;
        }
        index += 1;
    }
    output.map_or(Cow::Borrowed(code), Cow::Owned)
}

fn is_identifier_boundary(bytes: &[u8], index: usize) -> bool {
    bytes
        .get(index)
        .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
}

fn is_filter_position(bytes: &[u8], start: usize, end: usize) -> bool {
    let previous = bytes[..start]
        .iter()
        .rfind(|byte| !byte.is_ascii_whitespace())
        .copied();
    if matches!(previous, Some(b'.' | b'$')) {
        return false;
    }
    let next = bytes[end..]
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .copied();
    !matches!(next, Some(b'"' | b':'))
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
    let _resolution_guard = crate::runtime_resolution::activate();
    let pending_input_error = Rc::new(RefCell::new(None));
    let captured_input_error = Rc::clone(&pending_input_error);
    let inputs = inputs.map(move |result| match result {
        Ok(value) => {
            captured_input_error.borrow_mut().take();
            Ok(value)
        }
        Err(error) => {
            let error = error.into();
            let message = error.to_string();
            *captured_input_error.borrow_mut() = Some((message.clone(), error));
            Err(message)
        }
    });
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
        let input = input.map_err(|message| {
            E::from(
                take_input_error(&pending_input_error, &message)
                    .unwrap_or(YgError::InputParse(message)),
            )
        })?;
        context.activate_pending_input_meta();
        for output in filter.id.run((jaq_context.clone(), input)) {
            let output: YamlVal = output.map_err(|error| match error.get_err() {
                Ok(error) => {
                    let message = error.to_string();
                    E::from(
                        take_input_error(&pending_input_error, &message)
                            .or_else(|| {
                                crate::runtime_resolution::take_error().map(|error| {
                                    contextualize_runtime_error(
                                        context,
                                        YgError::runtime_resolution(error, message.clone()),
                                    )
                                })
                            })
                            .or_else(|| {
                                crate::runtime_limits::take_error().map(|error| {
                                    contextualize_runtime_error(context, pending_error(error))
                                })
                            })
                            .unwrap_or_else(|| {
                                contextualize_runtime_error(context, YgError::Runtime(message))
                            }),
                    )
                }
                Err(exception) => E::from(contextualize_runtime_error(
                    context,
                    YgError::runtime(exception),
                )),
            })?;
            if let Some(error) = crate::runtime_resolution::take_error() {
                let message = error.to_string();
                return Err(E::from(contextualize_runtime_error(
                    context,
                    YgError::runtime_resolution(error, message),
                )));
            }
            if let Some(error) = crate::runtime_limits::take_error() {
                return Err(E::from(contextualize_runtime_error(
                    context,
                    pending_error(error),
                )));
            }
            output
                .check_depth(&context.config.limits)
                .map_err(|error| {
                    E::from(contextualize_runtime_error(
                        context,
                        value_limit_error(error),
                    ))
                })?;
            last = Some(jaq_core::ValT::as_bool(&output));
            visit(output)?;
        }
        if let Some(error) = crate::runtime_resolution::take_error() {
            let message = error.to_string();
            return Err(E::from(contextualize_runtime_error(
                context,
                YgError::runtime_resolution(error, message),
            )));
        }
        if let Some(error) = crate::runtime_limits::take_error() {
            return Err(E::from(contextualize_runtime_error(
                context,
                pending_error(error),
            )));
        }
    }
    Ok(last)
}

fn pending_error(error: crate::runtime_limits::PendingError) -> YgError {
    match error {
        crate::runtime_limits::PendingError::Limit(error) => {
            YgError::runtime_limit(error, error.to_string())
        }
        crate::runtime_limits::PendingError::ResourceOverflow(resource) => {
            YgError::resource_overflow(resource, format!("{resource} overflow"))
        }
        crate::runtime_limits::PendingError::Message(message) => YgError::Runtime(message),
    }
}

fn value_limit_error(error: crate::value::ValueLimitError) -> YgError {
    match error {
        crate::value::ValueLimitError::Limit(error) => {
            YgError::runtime_limit(error, error.to_string())
        }
        crate::value::ValueLimitError::ResourceOverflow(resource) => {
            YgError::resource_overflow(resource, format!("{resource} overflow"))
        }
    }
}

fn take_input_error(
    pending: &RefCell<Option<(String, YgError)>>,
    message: &str,
) -> Option<YgError> {
    let mut pending = pending.borrow_mut();
    if pending
        .as_ref()
        .is_some_and(|(pending_message, _)| pending_message == message)
    {
        pending.take().map(|(_, error)| error)
    } else {
        None
    }
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
        YgError::RuntimeLimit { error, message } => {
            let metadata = context.input_meta.borrow();
            let source = metadata
                .filename
                .as_deref()
                .map_or_else(|| "<stdin>".to_owned(), str::to_owned);
            YgError::runtime_limit(
                error,
                format!(
                    "{source} document {}: {message}",
                    metadata.document_index + 1
                ),
            )
        }
        YgError::ResourceOverflow { resource, message } => {
            let metadata = context.input_meta.borrow();
            let source = metadata
                .filename
                .as_deref()
                .map_or_else(|| "<stdin>".to_owned(), str::to_owned);
            YgError::resource_overflow(
                resource,
                format!(
                    "{source} document {}: {message}",
                    metadata.document_index + 1
                ),
            )
        }
        YgError::RuntimeResolution { error, message } => {
            let metadata = context.input_meta.borrow();
            let source = metadata
                .filename
                .as_deref()
                .map_or_else(|| "<stdin>".to_owned(), str::to_owned);
            YgError::runtime_resolution(
                *error,
                format!(
                    "{source} document {}: {message}",
                    metadata.document_index + 1
                ),
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResolvedInput, resolution_handle};
    use yamalgam_resolve::MemoryRetriever;

    #[test]
    fn include_alias_rewrites_only_filter_positions() {
        assert_eq!(alias_include_filter("include"), "yginc__");
        assert_eq!(alias_include_filter(".x | include"), ".x | yginc__");
        assert_eq!(
            alias_include_filter("include \"module\"; ."),
            "include \"module\"; ."
        );
        assert_eq!(alias_include_filter(".include"), ".include");
        assert_eq!(alias_include_filter("$include"), "$include");
        assert_eq!(alias_include_filter("{include: .}"), "{include: .}");
        assert_eq!(
            alias_include_filter(r#""\(.x | include)""#),
            r#""\(.x | yginc__)""#
        );
        assert_eq!(
            alias_include_filter("\"include\" # include\n."),
            "\"include\" # include\n."
        );
    }

    #[test]
    fn resolution_session_keys_fail_closed_on_overflow() {
        let config = LoaderConfig::unchecked();
        let context = RunCtx::new(config.clone());
        context.next_resolution_key.set(u64::MAX);
        let input = || ResolvedInput {
            value: YamlVal::null(),
            resolution: resolution_handle(MemoryRetriever::default(), None, config.clone()),
        };

        for _ in 0..2 {
            assert!(matches!(
                context.register_resolved_input(input()),
                Err(YgError::ResourceOverflow {
                    resource: "resolution session key",
                    ..
                })
            ));
        }
        assert!(context.resolutions.borrow().is_empty());
        assert_eq!(context.next_resolution_key.get(), u64::MAX);
    }
}
