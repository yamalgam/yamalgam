//! Native yg filter registration.

use jaq_core::native::{Filter, Fun, bome, run, unary, v};
use jaq_core::{Error, RunPtr, ValR};

use crate::engine::YgKind;
use crate::value::{Data, YamlVal};

/// Return all native functions available to yg filters.
pub fn all_funs() -> impl Iterator<Item = Fun<YgKind>> {
    let run = run::<YgKind>;
    jaq_core::funs::<YgKind>()
        .chain(jaq_std::funs::<YgKind>())
        .chain(
            jaq_std::input::funs::<YgKind>()
                .into_vec()
                .into_iter()
                .map(run),
        )
        .chain(yg_natives().into_vec().into_iter().map(run))
}

fn yg_natives() -> Box<[Filter<RunPtr<YgKind>>]> {
    Box::new([
        ("_yaml_type", v(0), |context_value| {
            bome(Ok(YamlVal::str(context_value.1.type_name())))
        }),
        ("_infinite", v(0), |_| {
            bome(Ok(YamlVal::float(f64::INFINITY)))
        }),
        ("_nan", v(0), |_| bome(Ok(YamlVal::float(f64::NAN)))),
        ("_is_nan", v(0), |context_value| {
            bome(Ok(YamlVal::from(matches!(
                context_value.1.data,
                Data::Float(value) if value.is_nan()
            ))))
        }),
        ("_is_infinite", v(0), |context_value| {
            bome(Ok(YamlVal::from(matches!(
                context_value.1.data,
                Data::Float(value) if value.is_infinite()
            ))))
        }),
        ("_is_finite", v(0), |context_value| {
            bome(Ok(YamlVal::from(match context_value.1.data {
                Data::Int(_) => true,
                Data::Float(value) => value.is_finite(),
                _ => false,
            })))
        }),
        ("_is_normal", v(0), |context_value| {
            bome(Ok(YamlVal::from(match context_value.1.data {
                Data::Int(value) => value != 0,
                Data::Float(value) => value.is_normal(),
                _ => false,
            })))
        }),
        ("length", v(0), |context_value| {
            bome(context_value.1.length())
        }),
        ("has", v(1), |context_value| {
            unary(context_value, |value, key| {
                value.has(&key).map(YamlVal::from)
            })
        }),
        ("contains", v(1), |context_value| {
            unary(context_value, |value, contained| {
                Ok(YamlVal::from(value.contains(&contained)))
            })
        }),
        ("indices", v(1), |context_value| {
            unary(context_value, |value, needle| value.indices(&needle))
        }),
        ("bsearch", v(1), |context_value| {
            unary(context_value, |value, target| value.bsearch(&target))
        }),
        ("tojson", v(0), |context_value| {
            bome(Ok(YamlVal::str(crate::emit::to_json(
                &context_value.1,
                &crate::EmitOpts::compact(),
            ))))
        }),
        ("fromjson", v(0), |context_value| {
            bome(context_value.1.fromjson())
        }),
        ("tag", v(0), |context_value| {
            bome(Ok(context_value
                .1
                .tag_shorthand()
                .map_or_else(YamlVal::null, YamlVal::str)))
        }),
        ("anchor", v(0), |context_value| {
            bome(Ok(context_value
                .1
                .meta()
                .and_then(|metadata| metadata.anchor.as_deref().or(metadata.alias_of.as_deref()))
                .map_or_else(YamlVal::null, YamlVal::str)))
        }),
        ("style", v(0), |context_value| {
            bome(Ok(context_value
                .1
                .style_name()
                .map_or_else(YamlVal::null, YamlVal::str)))
        }),
        ("kind", v(0), |context_value| {
            bome(Ok(YamlVal::str(context_value.1.kind_name())))
        }),
        ("is_alias", v(0), |context_value| {
            bome(Ok(YamlVal::from(
                context_value
                    .1
                    .meta()
                    .is_some_and(|metadata| metadata.alias_of.is_some()),
            )))
        }),
        ("raw", v(0), |context_value| {
            bome(context_value.1.raw_text())
        }),
        ("set_tag", v(1), |context_value| {
            unary(context_value, YamlVal::set_tag)
        }),
        ("set_anchor", v(1), |context_value| {
            unary(context_value, YamlVal::set_anchor)
        }),
        ("set_style", v(1), |context_value| {
            unary(context_value, YamlVal::set_style)
        }),
        ("merge", v(0), |context_value| {
            bome(context_value.1.merge_keys())
        }),
        ("resolved_type", v(0), |context_value| {
            bome(Ok(YamlVal::str(context_value.1.type_name())))
        }),
        ("schema", v(0), |context_value| {
            bome(Ok(YamlVal::str(
                context_value.0.data().context.schema_name(),
            )))
        }),
        ("resolve", v(0), |context_value| {
            let schema = context_value.0.data().context.config.tag_resolution;
            bome(context_value.1.resolve_with(schema))
        }),
        ("resolve_as", v(1), |context_value| {
            unary(context_value, |value, schema| {
                value.resolve_with(parse_schema(&schema)?)
            })
        }),
        ("document_index", v(0), |context_value| {
            let index = context_value
                .0
                .data()
                .context
                .input_meta
                .borrow()
                .document_index;
            bome(Ok(YamlVal::from(index)))
        }),
        ("file_index", v(0), |context_value| {
            let index = context_value
                .0
                .data()
                .context
                .input_meta
                .borrow()
                .file_index;
            bome(Ok(YamlVal::from(index)))
        }),
        ("filename", v(0), |context_value| {
            let filename = context_value
                .0
                .data()
                .context
                .input_meta
                .borrow()
                .filename
                .clone();
            bome(Ok(filename.map_or_else(YamlVal::null, YamlVal::str)))
        }),
        ("explode_aliases", v(0), |context_value| {
            bome(Ok(context_value.1.explode_aliases()))
        }),
        ("aliases", v(0), |context_value| {
            bome(Ok(context_value.1.alias_report()))
        }),
        ("orphan_anchors", v(0), |context_value| {
            bome(Ok(context_value.1.orphan_anchors()))
        }),
    ])
}

fn parse_schema(value: &YamlVal) -> Result<yamalgam_core::TagResolution, Error<YamlVal>> {
    let Data::Str(name) = &value.data else {
        return Err(Error::typ(value.clone(), "schema name string"));
    };
    match name.as_ref() {
        "core" => Ok(yamalgam_core::TagResolution::Yaml12),
        "failsafe" => Ok(yamalgam_core::TagResolution::Failsafe),
        "json" => Ok(yamalgam_core::TagResolution::Json),
        "1.1" => Ok(yamalgam_core::TagResolution::Yaml11),
        _ => Err(Error::str(format_args!(
            "unknown schema {name}; expected core, failsafe, json, or 1.1"
        ))),
    }
}

impl YamlVal {
    fn length(&self) -> ValR<Self> {
        match &self.data {
            Data::Null => Ok(Self::int(0)),
            Data::Int(number) => Ok(number
                .checked_abs()
                .map_or_else(|| Self::float((*number as f64).abs()), Self::int)),
            Data::Float(number) => Ok(Self::float(number.abs())),
            Data::Str(string) => Ok(Self::from(string.chars().count())),
            Data::Seq(items) => Ok(Self::from(items.len())),
            Data::Map(mapping) => Ok(Self::from(mapping.0.len())),
            Data::Bool(_) => Err(Error::str(format_args!("{self} has no length"))),
        }
    }

    fn has(&self, key: &Self) -> Result<bool, Error<Self>> {
        match (&self.data, &key.data) {
            (Data::Seq(items), Data::Int(index)) => {
                Ok(usize::try_from(*index).is_ok_and(|index| index < items.len()))
            }
            (Data::Map(mapping), _) => Ok(mapping.0.contains_key(key)),
            _ => Err(Error::index(self.clone(), key.clone())),
        }
    }

    fn contains(&self, other: &Self) -> bool {
        match (&self.data, &other.data) {
            (Data::Str(left), Data::Str(right)) => left.contains(right.as_ref()),
            (Data::Seq(left), Data::Seq(right)) => right
                .iter()
                .all(|right| left.iter().any(|left| left.contains(right))),
            (Data::Map(left), Data::Map(right)) => right
                .0
                .iter()
                .all(|(key, right)| left.0.get(key).is_some_and(|left| left.contains(right))),
            _ => self == other,
        }
    }

    fn indices(&self, needle: &Self) -> ValR<Self> {
        let indices = match (&self.data, &needle.data) {
            (Data::Str(_), Data::Str(needle)) if needle.is_empty() => Vec::new(),
            (Data::Str(string), Data::Str(needle)) => string
                .char_indices()
                .enumerate()
                .filter_map(|(character_index, (byte_index, _))| {
                    string
                        .get(byte_index..byte_index.saturating_add(needle.len()))
                        .is_some_and(|candidate| candidate == needle.as_ref())
                        .then_some(Self::from(character_index))
                })
                .collect(),
            (Data::Seq(_), Data::Seq(needle)) if needle.is_empty() => Vec::new(),
            (Data::Seq(values), Data::Seq(needle)) => values
                .windows(needle.len())
                .enumerate()
                .filter_map(|(index, candidate)| {
                    (candidate == needle.as_slice()).then_some(Self::from(index))
                })
                .collect(),
            (Data::Seq(values), _) => values
                .iter()
                .enumerate()
                .filter_map(|(index, value)| (value == needle).then_some(Self::from(index)))
                .collect(),
            _ => return Err(Error::index(self.clone(), needle.clone())),
        };
        Ok(Self::seq(indices))
    }

    fn bsearch(&self, target: &Self) -> ValR<Self> {
        let Data::Seq(values) = &self.data else {
            return Err(Error::typ(self.clone(), "sequence"));
        };
        Ok(values.binary_search(target).map_or_else(
            |index| {
                i64::try_from(index).map_or_else(
                    |_| Self::float(-(index as f64) - 1.0),
                    |index| Self::int(-index - 1),
                )
            },
            Self::from,
        ))
    }

    fn fromjson(&self) -> ValR<Self> {
        let Data::Str(source) = &self.data else {
            return Err(Error::typ(self.clone(), "string"));
        };
        let config = yamalgam_core::LoaderConfig::moderate()
            .with_tag_resolution(yamalgam_core::TagResolution::Json);
        let mut documents = crate::build::parse_str_with_config(source, &config)
            .map_err(|error| Error::str(format_args!("fromjson: {error}")))?;
        if documents.len() == 1 {
            Ok(documents.remove(0))
        } else {
            Err(Error::str(format_args!(
                "fromjson: expected 1 document, got {}",
                documents.len()
            )))
        }
    }
}
