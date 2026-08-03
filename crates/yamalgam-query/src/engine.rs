//! jaq filter compilation and execution.

use std::cell::RefCell;
use std::rc::Rc;

use jaq_core::data::HasLut;
use jaq_core::load::{self, Arena, File, Loader};
use jaq_core::{Ctx, DataT, Filter, Lut, ValT as _, Vars, compile};
use jaq_std::input::{HasInputs, Inputs, RcIter};
use yamalgam_core::{LoaderConfig, TagResolution};

use crate::error::YgError;
use crate::value::YamlVal;

/// jaq data-kind marker for YAML values.
pub struct YgKind;

/// A query filter compiled for yg's YAML value model.
pub type CompiledFilter = Filter<YgKind>;

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
    /// Metadata updated as input documents are yielded.
    pub input_meta: RefCell<InputMeta>,
}

impl RunCtx {
    /// Construct a run context.
    #[must_use]
    pub fn new(config: LoaderConfig) -> Self {
        Self {
            config,
            input_meta: RefCell::new(InputMeta::default()),
        }
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

fn yg_defs() -> impl Iterator<Item = load::parse::Def<&'static str>> {
    load::parse(include_str!("defs.jq"), |parser| parser.defs())
        .expect("bundled yg definitions must parse")
        .into_iter()
}

/// Compile a query filter with the yg definitions and native functions.
pub fn compile_filter(code: &str, variables: &[String]) -> Result<CompiledFilter, YgError> {
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
        .with_funs(crate::funs::all_funs())
        .with_global_vars(variables.iter().map(String::as_str))
        .compile(modules)
        .map_err(|errors| YgError::FilterCompile(render_compile_errors(errors)))
}

/// Run a compiled filter over input values and visit every output.
///
/// The returned boolean is the truthiness of the last output, or `None` when
/// the filter produced no output.
pub fn run_filter<E: From<YgError>>(
    filter: &Filter<YgKind>,
    context: &RunCtx,
    inputs: impl Iterator<Item = Result<YamlVal, String>> + 'static,
    visit: impl FnMut(YamlVal) -> Result<(), E>,
) -> Result<Option<bool>, E> {
    run_filter_with_null_input(filter, context, inputs, false, visit)
}

/// Run a filter with the real input stream available to `input`/`inputs`.
///
/// When `null_input` is true, the filter driver is a single null while native
/// input filters continue to consume from `inputs`.
pub fn run_filter_with_null_input<E: From<YgError>>(
    filter: &Filter<YgKind>,
    context: &RunCtx,
    inputs: impl Iterator<Item = Result<YamlVal, String>> + 'static,
    null_input: bool,
    mut visit: impl FnMut(YamlVal) -> Result<(), E>,
) -> Result<Option<bool>, E> {
    let inputs: Box<dyn Iterator<Item = Result<YamlVal, String>>> = Box::new(inputs);
    let inputs = RcIter::new(inputs);
    let data = YgData {
        lut: &filter.lut,
        inputs: &inputs,
        context,
    };
    let jaq_context = Ctx::<YgKind>::new(&data, Vars::new([]));
    let mut last = None;

    let mut driver: Box<dyn Iterator<Item = Result<YamlVal, String>> + '_> = if null_input {
        Box::new(core::iter::once(Ok(YamlVal::null())))
    } else {
        Box::new(&inputs)
    };

    for input in &mut driver {
        let input = input.map_err(|error| E::from(YgError::InputParse(error)))?;
        for output in filter.id.run((jaq_context.clone(), input)) {
            let output: YamlVal = output.map_err(|error| E::from(YgError::runtime(error)))?;
            last = Some(output.as_bool());
            visit(output)?;
        }
    }
    Ok(last)
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
