//! Arithmetic operators with jq semantics.

use std::collections::BTreeSet;
use std::rc::Rc;

use jaq_core::{Error, ValR, ops::Math};

use crate::value::{Data, YamlVal, YgMap};

type Result = ValR<YamlVal>;

fn int_op(
    left: i64,
    right: i64,
    int: impl FnOnce(i64, i64) -> Option<i64>,
    float: impl FnOnce(f64, f64) -> f64,
) -> Data {
    int(left, right).map_or_else(|| Data::Float(float(left as f64, right as f64)), Data::Int)
}

const fn value(data: Data) -> YamlVal {
    YamlVal {
        data,
        meta: None,
        runtime_error: None,
    }
}

fn math_error(left: Data, op: Math, right: Data) -> Error<YamlVal> {
    Error::math(value(left), op, value(right))
}

fn deep_merge(left: &mut Rc<YgMap>, right: &YgMap) {
    let left = Rc::make_mut(left);
    for (key, right_value) in &right.0 {
        match (left.0.get_mut(key), &right_value.data) {
            (
                Some(YamlVal {
                    data: Data::Map(left_map),
                    ..
                }),
                Data::Map(right_map),
            ) => deep_merge(left_map, right_map),
            (Some(left_value), _) => *left_value = right_value.clone(),
            (None, _) => {
                left.0.insert(key.clone(), right_value.clone());
            }
        }
    }
}

// r[impl diverge.add]
// r[impl diverge.add-strings]
// r[impl diverge.add-mappings]
impl core::ops::Add for YamlVal {
    type Output = Result;

    fn add(self, rhs: Self) -> Self::Output {
        use Data::{Float, Int, Map, Null, Seq, Str};

        let data =
            match (self.data, rhs.data) {
                (Null, data) | (data, Null) => data,
                (Int(left), Int(right)) => {
                    int_op(left, right, i64::checked_add, |left, right| left + right)
                }
                (Int(left), Float(right)) => Float(left as f64 + right),
                (Float(left), Int(right)) => Float(left + right as f64),
                (Float(left), Float(right)) => Float(left + right),
                (Str(left), Str(right)) => {
                    let size = left
                        .len()
                        .checked_add(right.len())
                        .ok_or_else(|| crate::runtime_limits::jaq_overflow_error("string size"))?;
                    crate::runtime_limits::check_scalar_size(size)
                        .map_err(crate::runtime_limits::jaq_limit_error)?;
                    let mut output = String::with_capacity(size);
                    output.push_str(&left);
                    output.push_str(&right);
                    Str(Rc::from(output))
                }
                (Seq(mut left), Seq(right)) => {
                    let size = left.len().checked_add(right.len()).ok_or_else(|| {
                        crate::runtime_limits::jaq_overflow_error("collection size")
                    })?;
                    crate::runtime_limits::check_collection_size(size)
                        .map_err(crate::runtime_limits::jaq_limit_error)?;
                    Rc::make_mut(&mut left).extend(right.iter().cloned());
                    Seq(left)
                }
                (Map(mut left), Map(right)) => {
                    let added = right
                        .0
                        .keys()
                        .filter(|key| !left.0.contains_key(*key))
                        .count();
                    let size = left.0.len().checked_add(added).ok_or_else(|| {
                        crate::runtime_limits::jaq_overflow_error("collection size")
                    })?;
                    crate::runtime_limits::check_collection_size(size)
                        .map_err(crate::runtime_limits::jaq_limit_error)?;
                    Rc::make_mut(&mut left).0.extend(
                        right
                            .0
                            .iter()
                            .map(|(key, value)| (key.clone(), value.clone())),
                    );
                    Map(left)
                }
                (left, right) => return Err(math_error(left, Math::Add, right)),
            };
        Ok(value(data))
    }
}

impl core::ops::Sub for YamlVal {
    type Output = Result;

    fn sub(self, rhs: Self) -> Self::Output {
        use Data::{Float, Int, Seq};

        let data = match (self.data, rhs.data) {
            (Int(left), Int(right)) => {
                int_op(left, right, i64::checked_sub, |left, right| left - right)
            }
            (Int(left), Float(right)) => Float(left as f64 - right),
            (Float(left), Int(right)) => Float(left - right as f64),
            (Float(left), Float(right)) => Float(left - right),
            (Seq(mut left), Seq(right)) => {
                let right = right.iter().collect::<BTreeSet<_>>();
                Rc::make_mut(&mut left).retain(|item| !right.contains(item));
                Seq(left)
            }
            (left, right) => return Err(math_error(left, Math::Sub, right)),
        };
        Ok(value(data))
    }
}

// r[impl diverge.multiply]
impl core::ops::Mul for YamlVal {
    type Output = Result;

    fn mul(self, rhs: Self) -> Self::Output {
        use Data::{Float, Int, Map, Null, Str};

        let data = match (self.data, rhs.data) {
            (Int(left), Int(right)) => {
                int_op(left, right, i64::checked_mul, |left, right| left * right)
            }
            (Int(left), Float(right)) => Float(left as f64 * right),
            (Float(left), Int(right)) => Float(left * right as f64),
            (Float(left), Float(right)) => Float(left * right),
            (Str(string), Int(count)) | (Int(count), Str(string)) if count > 0 => {
                let count = usize::try_from(count)
                    .map_err(|_| math_error(Str(string.clone()), Math::Mul, Int(count)))?;
                let size = string
                    .len()
                    .checked_mul(count)
                    .ok_or_else(|| crate::runtime_limits::jaq_overflow_error("string size"))?;
                crate::runtime_limits::check_scalar_size(size)
                    .map_err(crate::runtime_limits::jaq_limit_error)?;
                Str(Rc::from(string.repeat(count)))
            }
            (Str(_), Int(_)) | (Int(_), Str(_)) => Null,
            (Map(mut left), Map(right)) => {
                deep_merge(&mut left, &right);
                Map(left)
            }
            (left, right) => return Err(math_error(left, Math::Mul, right)),
        };
        Ok(value(data))
    }
}

impl core::ops::Div for YamlVal {
    type Output = Result;

    fn div(self, rhs: Self) -> Self::Output {
        use Data::{Float, Int, Str};

        let data = match (self.data, rhs.data) {
            (left, right @ Int(0)) | (left, right @ Float(0.0)) => {
                return Err(math_error(left, Math::Div, right));
            }
            (Int(left), Int(right))
                if right != 0 && left.checked_rem(right).is_some_and(|rem| rem == 0) =>
            {
                left.checked_div(right)
                    .map_or_else(|| Float(left as f64 / right as f64), Int)
            }
            (Int(left), Int(right)) => Float(left as f64 / right as f64),
            (Int(left), Float(right)) => Float(left as f64 / right),
            (Float(left), Int(right)) => Float(left / right as f64),
            (Float(left), Float(right)) => Float(left / right),
            (Str(string), Str(separator)) => {
                let parts = if string.is_empty() {
                    Vec::new()
                } else if separator.is_empty() {
                    string
                        .chars()
                        .map(|c| Self::str(c.to_string()))
                        .collect::<Vec<_>>()
                } else {
                    string
                        .split(separator.as_ref())
                        .map(Self::str)
                        .collect::<Vec<_>>()
                };
                return Ok(Self::seq(parts));
            }
            (left, right) => return Err(math_error(left, Math::Div, right)),
        };
        Ok(value(data))
    }
}

impl core::ops::Rem for YamlVal {
    type Output = Result;

    fn rem(self, rhs: Self) -> Self::Output {
        use Data::{Float, Int};

        let data = match (self.data, rhs.data) {
            (left, right @ Int(0)) | (left, right @ Float(0.0)) => {
                return Err(math_error(left, Math::Rem, right));
            }
            (Int(left), Int(right)) if right != 0 => Int(left.checked_rem(right).unwrap_or(0)),
            (Int(left), Float(right)) => Float(left as f64 % right),
            (Float(left), Int(right)) => Float(left % right as f64),
            (Float(left), Float(right)) => Float(left % right),
            (left, right) => return Err(math_error(left, Math::Rem, right)),
        };
        Ok(value(data))
    }
}

impl core::ops::Neg for YamlVal {
    type Output = Result;

    fn neg(self) -> Self::Output {
        let data = match self.data {
            Data::Int(number) => number
                .checked_neg()
                .map_or_else(|| Data::Float(-(number as f64)), Data::Int),
            Data::Float(number) => Data::Float(-number),
            data => return Err(Error::typ(value(data), "number")),
        };
        Ok(value(data))
    }
}

#[cfg(test)]
mod tests {
    use crate::build::parse_str;
    use crate::value::YamlVal;

    #[test]
    fn add_semantics() {
        let add = |left: YamlVal, right: YamlVal| (left + right).unwrap();

        assert_eq!(add(YamlVal::null(), YamlVal::int(2)), YamlVal::int(2));
        assert_eq!(
            add(YamlVal::int(1), YamlVal::float(0.5)),
            YamlVal::float(1.5)
        );
        assert_eq!(
            add(YamlVal::str("a"), YamlVal::str("b")),
            YamlVal::str("ab")
        );
        assert_eq!(
            add(
                YamlVal::seq(vec![YamlVal::int(1)]),
                YamlVal::seq(vec![YamlVal::int(2)])
            ),
            YamlVal::seq(vec![YamlVal::int(1), YamlVal::int(2)])
        );
        assert!((YamlVal::from(true) + YamlVal::int(1)).is_err());
    }

    #[test]
    fn int_overflow_promotes_to_float() {
        let result = (YamlVal::int(i64::MAX) + YamlVal::int(1)).unwrap();
        assert_eq!(result, YamlVal::float(i64::MAX as f64 + 1.0));

        let result = (-YamlVal::int(i64::MIN)).unwrap();
        assert_eq!(result, YamlVal::float(-(i64::MIN as f64)));
    }

    #[test]
    fn collection_add_and_subtract_follow_jq() {
        let left = parse_str("{a: 1, b: 2}\n").unwrap().remove(0);
        let right = parse_str("{b: 9, c: 3}\n").unwrap().remove(0);
        let merged = (left + right).unwrap();
        assert_eq!(merged.to_string(), "{a: 1, b: 9, c: 3}");

        let left = YamlVal::seq(vec![YamlVal::int(1), YamlVal::int(2), YamlVal::int(1)]);
        let right = YamlVal::seq(vec![YamlVal::int(1)]);
        assert_eq!((left - right).unwrap(), YamlVal::seq(vec![YamlVal::int(2)]));
    }

    #[test]
    fn mul_deep_merges_mappings() {
        let left = parse_str("x: {y: 1, z: 2}\n").unwrap().remove(0);
        let right = parse_str("x: {y: 9}\n").unwrap().remove(0);
        let merged = (left * right).unwrap();

        assert_eq!(
            merged.get_str("x").unwrap().get_str("y"),
            Some(&YamlVal::int(9))
        );
        assert_eq!(
            merged.get_str("x").unwrap().get_str("z"),
            Some(&YamlVal::int(2))
        );
    }

    #[test]
    fn string_multiplication_follows_jq() {
        assert_eq!(
            (YamlVal::str("ab") * YamlVal::int(3)).unwrap(),
            YamlVal::str("ababab")
        );
        assert_eq!(
            (YamlVal::int(0) * YamlVal::str("ab")).unwrap(),
            YamlVal::null()
        );
    }

    #[test]
    fn division_and_remainder_follow_jq() {
        assert_eq!(
            (YamlVal::int(10) / YamlVal::int(5)).unwrap(),
            YamlVal::int(2)
        );
        assert_eq!(
            (YamlVal::int(5) / YamlVal::int(2)).unwrap(),
            YamlVal::float(2.5)
        );
        assert!((YamlVal::int(5) / YamlVal::int(0)).is_err());
        assert!((YamlVal::float(5.0) / YamlVal::float(0.0)).is_err());
        assert_eq!(
            (YamlVal::int(5) % YamlVal::int(2)).unwrap(),
            YamlVal::int(1)
        );
        assert!((YamlVal::int(5) % YamlVal::int(0)).is_err());
    }

    #[test]
    fn div_splits_strings() {
        assert_eq!(
            (YamlVal::str("a,b") / YamlVal::str(",")).unwrap(),
            YamlVal::seq(vec![YamlVal::str("a"), YamlVal::str("b")])
        );
        assert_eq!(
            (YamlVal::str("hé") / YamlVal::str("")).unwrap(),
            YamlVal::seq(vec![YamlVal::str("h"), YamlVal::str("é")])
        );
        assert_eq!(
            (YamlVal::str("") / YamlVal::str(",")).unwrap(),
            YamlVal::seq(Vec::new())
        );
    }
}
