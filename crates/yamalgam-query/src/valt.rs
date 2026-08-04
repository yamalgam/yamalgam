//! jaq value-trait implementations.

use std::rc::Rc;

use jaq_core::box_iter::box_once;
use jaq_core::path::Opt;
use jaq_core::val::{Range, ValR, ValX};
use jaq_core::{Error, Exn};

use crate::value::{Data, YamlVal, YgMap, YgSeq};

impl From<Range<Self>> for YamlVal {
    fn from(range: Range<Self>) -> Self {
        let pairs = [("start", range.start), ("end", range.end)]
            .into_iter()
            .filter_map(|(key, value)| value.map(|value| (Self::str(key), value)))
            .collect();
        Self::map(pairs)
    }
}

impl jaq_core::ValT for YamlVal {
    fn from_num(number: &str) -> ValR<Self> {
        number.parse::<i64>().map(Self::int).or_else(|_| {
            number
                .parse::<f64>()
                .map(Self::float)
                .map_err(|_| Error::str(format_args!("cannot parse {number} as number")))
        })
    }

    // r[impl filters.construct.mapping]
    fn from_map<I: IntoIterator<Item = (Self, Self)>>(iter: I) -> ValR<Self> {
        let mut mapping = YgMap::default();
        for (key, value) in iter {
            if !mapping.0.contains_key(&key) {
                let size = mapping
                    .0
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| Error::str("collection size overflow"))?;
                crate::runtime_limits::check_collection_size(size).map_err(Error::str)?;
            }
            mapping.0.insert(key, value);
        }
        Ok(yaml_value(Data::Map(Rc::new(mapping))))
    }

    // r[impl filters.iterator]
    // r[impl filters.recurse]
    // r[impl diverge.entries]
    fn key_values(self) -> Box<dyn Iterator<Item = ValR<(Self, Self), Self>>> {
        match self.data {
            Data::Seq(items) => Box::new(
                Rc::unwrap_or_clone(items)
                    .into_values()
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| Ok((Self::from(index), value))),
            ),
            Data::Map(mapping) => Box::new(
                Rc::unwrap_or_clone(mapping)
                    .into_entries()
                    .into_iter()
                    .map(Ok),
            ),
            data => box_once(Err(Error::typ(
                yaml_value(data),
                "iterable (sequence or mapping)",
            ))),
        }
    }

    // r[impl filters.iterator]
    // r[impl filters.recurse]
    fn values(self) -> Box<dyn Iterator<Item = ValR<Self>>> {
        match self.data {
            Data::Seq(items) => {
                Box::new(Rc::unwrap_or_clone(items).into_values().into_iter().map(Ok))
            }
            Data::Map(mapping) => Box::new(
                Rc::unwrap_or_clone(mapping)
                    .into_entries()
                    .into_iter()
                    .map(|(_, value)| Ok(value)),
            ),
            data => box_once(Err(Error::typ(
                yaml_value(data),
                "iterable (sequence or mapping)",
            ))),
        }
    }

    // r[impl filters.field]
    // r[impl filters.field.nonstring-keys]
    // r[impl filters.index]
    // r[impl filters.slice]
    // r[impl filters.optional]
    fn index(self, index: &Self) -> ValR<Self> {
        if matches!(&self.data, Data::Seq(_) | Data::Str(_))
            && let Data::Map(range) = &index.data
        {
            let start = range.0.get(&Self::str("start"));
            let end = range.0.get(&Self::str("end"));
            return self.range(start..end);
        }

        if let Data::Seq(items) = &self.data
            && let Some(position) = index.as_index()
        {
            return Ok(abs_index(position, items.len())
                .and_then(|position| items.get(position).cloned())
                .unwrap_or_default());
        }

        match (&self.data, &index.data) {
            (Data::Null, _) => Ok(Self::null()),
            (Data::Seq(_), Data::Seq(needle)) if needle.is_empty() => Ok(Self::seq(Vec::new())),
            (Data::Seq(items), Data::Seq(needle)) => Ok(items
                .windows(needle.len())
                .enumerate()
                .filter_map(|(index, window)| {
                    (window == needle.as_slice()).then_some(Self::from(index))
                })
                .collect()),
            (Data::Map(mapping), _) => Ok(mapping.0.get(index).cloned().unwrap_or_default()),
            _ => Err(Error::index(self, index.clone())),
        }
    }

    // r[impl filters.slice]
    fn range(self, range: Range<&Self>) -> ValR<Self> {
        let (start, end) = range_bounds(range)?;
        match self.data {
            Data::Seq(items) => {
                let (skip, take) = skip_take(start, end, items.len());
                Ok(Self::seq(
                    items.iter().skip(skip).take(take).cloned().collect(),
                ))
            }
            Data::Str(string) => {
                let (skip, take) = skip_take(start, end, string.chars().count());
                Ok(Self::str(
                    string.chars().skip(skip).take(take).collect::<String>(),
                ))
            }
            data => Err(Error::typ(
                yaml_value(data),
                "rangeable (sequence or string)",
            )),
        }
    }

    fn map_values<'a, I: Iterator<Item = ValX<'a, Self>>>(
        mut self,
        opt: Opt,
        f: impl Fn(Self) -> I,
    ) -> ValX<'a, Self> {
        match self.data_mut() {
            Data::Seq(items) => {
                let previous = core::mem::take(&mut Rc::make_mut(items).0);
                let mut updated = Vec::new();
                for value in previous {
                    for result in f(value) {
                        let value = result?;
                        let size = updated
                            .len()
                            .checked_add(1)
                            .ok_or_else(|| Exn::from(Error::str("collection size overflow")))?;
                        crate::runtime_limits::check_collection_size(size)
                            .map_err(Error::str)
                            .map_err(Exn::from)?;
                        updated.push(value);
                    }
                }
                *Rc::make_mut(items) = YgSeq(updated);
                Ok(self)
            }
            Data::Map(mapping) => {
                let previous = core::mem::take(&mut Rc::make_mut(mapping).0);
                let mut updated = YgMap::default();
                for (key, value) in previous {
                    if let Some(value) = f(value).next() {
                        let value = value?;
                        let size = updated
                            .0
                            .len()
                            .checked_add(1)
                            .ok_or_else(|| Exn::from(Error::str("collection size overflow")))?;
                        crate::runtime_limits::check_collection_size(size)
                            .map_err(Error::str)
                            .map_err(Exn::from)?;
                        updated.0.insert(key, value);
                    }
                }
                *Rc::make_mut(mapping) = updated;
                Ok(self)
            }
            _ => opt.fail(self, |value| {
                Exn::from(Error::typ(value, "iterable (sequence or mapping)"))
            }),
        }
    }

    // r[impl filters.assign.update]
    // r[impl filters.assign.plain]
    // r[impl filters.assign.arithmetic]
    // r[impl phases.mutation]
    fn map_index<'a, I: Iterator<Item = ValX<'a, Self>>>(
        mut self,
        index: &Self,
        opt: Opt,
        f: impl Fn(Self) -> I,
    ) -> ValX<'a, Self> {
        if matches!(&self.data, Data::Seq(_) | Data::Str(_))
            && let Data::Map(range) = &index.data
        {
            let start = range.0.get(&Self::str("start"));
            let end = range.0.get(&Self::str("end"));
            return self.map_range(start..end, opt, f);
        }

        if matches!(&self.data, Data::Null) {
            let replacement = f(Self::null()).next().transpose()?;
            let Some(replacement) = replacement else {
                return Ok(self);
            };
            let data = match index.as_index() {
                Some(position) if position >= 0 => {
                    let Ok(position) = usize::try_from(position) else {
                        let error = Error::str(format_args!("index {index} out of bounds"));
                        return opt.fail(self, |_| Exn::from(error));
                    };
                    let Some(size) = position.checked_add(1) else {
                        let error = Error::str(format_args!("index {index} out of bounds"));
                        return opt.fail(self, |_| Exn::from(error));
                    };
                    if let Err(message) = crate::runtime_limits::check_collection_size(size) {
                        let error = Error::str(message);
                        return opt.fail(self, |_| Exn::from(error));
                    }
                    let mut items = vec![Self::null(); position];
                    items.push(replacement);
                    Data::Seq(Rc::new(crate::value::YgSeq(items)))
                }
                Some(_) => {
                    let error = Error::str(format_args!("index {index} out of bounds"));
                    return opt.fail(self, |_| Exn::from(error));
                }
                None => Data::Map(Rc::new(crate::value::YgMap(
                    core::iter::once((index.clone(), replacement)).collect(),
                ))),
            };
            *self.data_mut() = data;
            return Ok(self);
        }

        match self.data_mut() {
            Data::Map(mapping) => {
                use indexmap::map::Entry::{Occupied, Vacant};

                match Rc::make_mut(mapping).0.entry(index.clone()) {
                    Occupied(mut entry) => {
                        let previous = core::mem::take(entry.get_mut());
                        match f(previous).next().transpose()? {
                            Some(value) => {
                                entry.insert(value);
                            }
                            None => {
                                entry.shift_remove();
                            }
                        }
                    }
                    Vacant(entry) => {
                        if let Some(value) = f(Self::null()).next().transpose()? {
                            entry.insert(value);
                        }
                    }
                }
                Ok(self)
            }
            Data::Seq(items) => {
                let Some(requested) = index.as_index() else {
                    let error = Error::str(format_args!("index {index} out of bounds"));
                    return opt.fail(self, |_| Exn::from(error));
                };

                let position = abs_index(requested, items.len());
                if position.is_none() && requested >= 0 {
                    let replacement = f(Self::null()).next().transpose()?;
                    let Some(replacement) = replacement else {
                        return Ok(self);
                    };
                    let Ok(position) = usize::try_from(requested) else {
                        let error = Error::str(format_args!("index {index} out of bounds"));
                        return opt.fail(self, |_| Exn::from(error));
                    };
                    let Some(size) = position.checked_add(1) else {
                        let error = Error::str(format_args!("index {index} out of bounds"));
                        return opt.fail(self, |_| Exn::from(error));
                    };
                    if let Err(message) = crate::runtime_limits::check_collection_size(size) {
                        let error = Error::str(message);
                        return opt.fail(self, |_| Exn::from(error));
                    }
                    let items = Rc::make_mut(items);
                    items.resize(size, Self::null());
                    items[position] = replacement;
                    return Ok(self);
                }
                let Some(position) = position else {
                    let error = Error::str(format_args!("index {index} out of bounds"));
                    return opt.fail(self, |_| Exn::from(error));
                };

                let items = Rc::make_mut(items);
                let previous = core::mem::take(&mut items[position]);
                match f(previous).next().transpose()? {
                    Some(value) => items[position] = value,
                    None => {
                        items.remove(position);
                    }
                }
                Ok(self)
            }
            _ => opt.fail(self, |value| {
                Exn::from(Error::typ(value, "iterable (sequence or mapping)"))
            }),
        }
    }

    fn map_range<'a, I: Iterator<Item = ValX<'a, Self>>>(
        mut self,
        range: Range<&Self>,
        opt: Opt,
        f: impl Fn(Self) -> I,
    ) -> ValX<'a, Self> {
        let (start, end) = match range_bounds(range) {
            Ok(bounds) => bounds,
            Err(error) => return opt.fail(self, |_| Exn::from(error)),
        };

        match self.data_mut() {
            Data::Seq(items) => {
                let (skip, take) = skip_take(start, end, items.len());
                let slice = Self::seq(items.iter().skip(skip).take(take).cloned().collect());
                let replacement = f(slice).next().transpose()?;
                let replacement = match replacement {
                    None => Vec::new(),
                    Some(value) => value
                        .into_seq_vec()
                        .map_err(|value| Exn::from(Error::typ(value, "sequence")))?,
                };
                Rc::make_mut(items).splice(skip..skip + take, replacement);
                Ok(self)
            }
            Data::Str(string) => {
                let (skip, take) = skip_take(start, end, string.chars().count());
                let (start_byte, end_byte) = char_byte_range(string, skip, take);
                let slice = Self::str(&string[start_byte..end_byte]);
                let replacement = match f(slice).next().transpose()? {
                    None => String::new(),
                    Some(value) => value
                        .into_rust_string()
                        .map_err(|value| Exn::from(Error::typ(value, "string")))?,
                };
                let mut updated = String::with_capacity(
                    string.len() - (end_byte - start_byte) + replacement.len(),
                );
                updated.push_str(&string[..start_byte]);
                updated.push_str(&replacement);
                updated.push_str(&string[end_byte..]);
                *string = Rc::from(updated);
                Ok(self)
            }
            _ => opt.fail(self, |value| {
                Exn::from(Error::typ(value, "rangeable (sequence or string)"))
            }),
        }
    }

    fn as_bool(&self) -> bool {
        !matches!(&self.data, Data::Null | Data::Bool(false))
    }

    fn into_string(self) -> Self {
        match self.data {
            Data::Str(string) => Self::str(string),
            data => crate::emit::to_json(&yaml_value(data), &crate::EmitOpts::compact())
                .map_or_else(|error| Self::str(format!("<yg: {error}>")), Self::str),
        }
    }
}

impl YamlVal {
    pub(crate) fn as_index(&self) -> Option<i64> {
        match self.data {
            Data::Int(number) => Some(number),
            Data::Float(number)
                if number.is_finite()
                    && number.fract() == 0.0
                    && number >= i64::MIN as f64
                    && number < -(i64::MIN as f64) =>
            {
                Some(number as i64)
            }
            _ => None,
        }
    }

    fn into_rust_string(self) -> Result<String, Self> {
        match self.data {
            Data::Str(string) => Ok(string.to_string()),
            _ => Err(self),
        }
    }
}

const fn yaml_value(data: Data) -> YamlVal {
    YamlVal { data, meta: None }
}

fn range_bounds(range: Range<&YamlVal>) -> ValR<(Option<i64>, Option<i64>), YamlVal> {
    let bound = |value: Option<&YamlVal>| {
        value
            .filter(|value| !matches!(&value.data, Data::Null))
            .map(|value| {
                value
                    .as_index()
                    .ok_or_else(|| Error::typ(value.clone(), "integer"))
            })
            .transpose()
    };
    Ok((bound(range.start)?, bound(range.end)?))
}

fn abs_index(index: i64, len: usize) -> Option<usize> {
    let len = i128::try_from(len).ok()?;
    let index = i128::from(index);
    let index = if index < 0 { index + len } else { index };
    usize::try_from(index)
        .ok()
        .filter(|index| *index < len as usize)
}

fn skip_take(start: Option<i64>, end: Option<i64>, len: usize) -> (usize, usize) {
    let len = i128::try_from(len).unwrap_or(i128::MAX);
    let absolute = |index: i64| {
        let index = i128::from(index);
        let index = if index < 0 { index + len } else { index };
        usize::try_from(index.clamp(0, len)).unwrap_or(usize::MAX)
    };
    let start = start.map_or(0, absolute);
    let end = end.map_or_else(|| usize::try_from(len).unwrap_or(usize::MAX), absolute);
    (start, end.saturating_sub(start))
}

fn char_byte_range(string: &str, skip: usize, take: usize) -> (usize, usize) {
    let byte_offset = |character| {
        string
            .char_indices()
            .nth(character)
            .map_or(string.len(), |(offset, _)| offset)
    };
    (byte_offset(skip), byte_offset(skip.saturating_add(take)))
}

impl jaq_std::ValT for YamlVal {
    fn into_seq<S: FromIterator<Self>>(self) -> Result<S, Self> {
        self.into_seq_vec()
            .map(|values| values.into_iter().collect())
    }

    fn is_int(&self) -> bool {
        matches!(self.data, Data::Int(_))
    }

    fn as_isize(&self) -> Option<isize> {
        let Data::Int(number) = self.data else {
            return None;
        };
        isize::try_from(number).ok()
    }

    fn as_f64(&self) -> Option<f64> {
        match &self.data {
            Data::Int(number) => Some(*number as f64),
            Data::Float(number) => Some(*number),
            _ => None,
        }
    }

    fn is_utf8_str(&self) -> bool {
        matches!(&self.data, Data::Str(_))
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match &self.data {
            Data::Str(string) => Some(string.as_bytes()),
            _ => None,
        }
    }

    fn as_sub_str(&self, sub: &[u8]) -> Self {
        debug_assert!(matches!(&self.data, Data::Str(_)));
        Self::str(String::from_utf8_lossy(sub).into_owned())
    }

    fn from_utf8_bytes(bytes: impl AsRef<[u8]> + Send + 'static) -> Self {
        Self::str(String::from_utf8_lossy(bytes.as_ref()).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use jaq_core::{ValT as _, path::Opt};

    use crate::build::parse_str;
    use crate::value::YamlVal;

    #[test]
    fn from_num_parses_ints_and_floats() {
        assert_eq!(YamlVal::from_num("3").unwrap(), YamlVal::int(3));
        assert_eq!(YamlVal::from_num("3.5").unwrap(), YamlVal::float(3.5));
        assert_eq!(YamlVal::from_num("1e3").unwrap(), YamlVal::float(1000.0));
    }

    #[test]
    fn index_supports_any_mapping_key_type() {
        let mapping = parse_str("1: int-key\ntrue: bool-key\n").unwrap().remove(0);

        assert_eq!(
            mapping.clone().index(&YamlVal::int(1)).unwrap(),
            YamlVal::str("int-key")
        );
        assert_eq!(
            mapping.index(&YamlVal::from(true)).unwrap(),
            YamlVal::str("bool-key")
        );
    }

    #[test]
    fn missing_index_is_null_and_negative_sequence_index_wraps() {
        let mapping = parse_str("a: 1\n").unwrap().remove(0);
        assert_eq!(
            mapping.index(&YamlVal::str("missing")).unwrap(),
            YamlVal::null()
        );

        let sequence = parse_str("[1, 2, 3]\n").unwrap().remove(0);
        assert_eq!(sequence.index(&YamlVal::int(-1)).unwrap(), YamlVal::int(3));
    }

    #[test]
    fn values_iterates_sequence_and_mapping_values() {
        let mapping = parse_str("a: 1\nb: 2\n").unwrap().remove(0);
        let values = mapping.values().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(values, vec![YamlVal::int(1), YamlVal::int(2)]);
    }

    #[test]
    fn map_values_preserves_container_metadata() {
        let mapping = parse_str("{a: 1}\n").unwrap().remove(0);
        let style = mapping.meta().unwrap().style;
        let output = mapping
            .map_values(Opt::Essential, |value| core::iter::once(Ok(value)))
            .unwrap();

        assert_eq!(output.meta().unwrap().style, style);
    }

    #[test]
    fn map_index_builds_and_extends_sequences_for_nonnegative_integer_paths() {
        let built = YamlVal::null()
            .map_index(&YamlVal::int(2), Opt::Essential, |_| {
                core::iter::once(Ok(YamlVal::int(7)))
            })
            .unwrap();
        assert_eq!(
            built,
            YamlVal::seq(vec![YamlVal::null(), YamlVal::null(), YamlVal::int(7)])
        );

        let extended = YamlVal::seq(vec![YamlVal::int(1)])
            .map_index(&YamlVal::int(2), Opt::Essential, |_| {
                core::iter::once(Ok(YamlVal::int(3)))
            })
            .unwrap();
        assert_eq!(
            extended,
            YamlVal::seq(vec![YamlVal::int(1), YamlVal::null(), YamlVal::int(3)])
        );
    }

    #[test]
    fn map_index_builds_mappings_and_ignores_missing_deletes() {
        let built = YamlVal::null()
            .map_index(&YamlVal::str("a"), Opt::Essential, |_| {
                core::iter::once(Ok(YamlVal::int(1)))
            })
            .unwrap();
        assert_eq!(
            built,
            YamlVal::map(vec![(YamlVal::str("a"), YamlVal::int(1))])
        );

        let unchanged = YamlVal::seq(vec![YamlVal::int(1)])
            .map_index(&YamlVal::int(3), Opt::Essential, |_| core::iter::empty())
            .unwrap();
        assert_eq!(unchanged, YamlVal::seq(vec![YamlVal::int(1)]));
    }

    #[test]
    fn ranges_slice_sequences_and_strings_by_value_units() {
        let sequence = parse_str("[0, 1, 2, 3]\n").unwrap().remove(0);
        assert_eq!(
            sequence
                .range(Some(&YamlVal::int(1))..Some(&YamlVal::int(-1)))
                .unwrap(),
            YamlVal::seq(vec![YamlVal::int(1), YamlVal::int(2)])
        );

        let string = YamlVal::str("héllo");
        assert_eq!(
            string
                .range(Some(&YamlVal::int(1))..Some(&YamlVal::int(3)))
                .unwrap(),
            YamlVal::str("él")
        );
    }

    #[test]
    fn map_range_updates_unicode_strings_and_preserves_metadata() {
        let string = parse_str("'héllo'\n").unwrap().remove(0);
        let style = string.meta().unwrap().style;
        let output = string
            .map_range(
                Some(&YamlVal::int(1))..Some(&YamlVal::int(3)),
                Opt::Essential,
                |_| core::iter::once(Ok(YamlVal::str("EY"))),
            )
            .unwrap();

        assert_eq!(output, YamlVal::str("hEYlo"));
        assert_eq!(output.meta().unwrap().style, style);
    }

    #[test]
    fn jq_truthiness_and_interpolation_are_supported() {
        assert!(!jaq_core::ValT::as_bool(&YamlVal::null()));
        assert!(!jaq_core::ValT::as_bool(&YamlVal::from(false)));
        assert!(jaq_core::ValT::as_bool(&YamlVal::int(0)));
        assert_eq!(YamlVal::int(3).into_string(), YamlVal::str("3"));
        assert_eq!(
            YamlVal::seq(vec![YamlVal::str("foo")]).into_string(),
            YamlVal::str("[\"foo\"]")
        );
    }

    #[test]
    fn std_valt_exposes_sequences_numbers_and_strings() {
        use jaq_std::ValT as _;

        assert!(YamlVal::int(3).is_int());
        assert!(!YamlVal::float(3.0).is_int());
        assert_eq!(YamlVal::int(3).as_isize(), Some(3));
        assert_eq!(YamlVal::float(3.0).as_isize(), None);
        assert_eq!(YamlVal::float(3.5).as_f64(), Some(3.5));
        assert!(YamlVal::str("x").is_utf8_str());
        assert_eq!(YamlVal::str("abc").as_bytes(), Some(b"abc".as_ref()));
        assert_eq!(
            YamlVal::seq(vec![YamlVal::int(1)])
                .into_seq::<Vec<_>>()
                .unwrap(),
            vec![YamlVal::int(1)]
        );
    }
}
