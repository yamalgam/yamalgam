#![allow(missing_docs)]

//! Robustness against `Deserialize` impls that violate the serde access
//! contracts. These are safe-Rust callers, so the deserializer must fail
//! with an error — never panic (debug) or loop on wrapped counters (release).

use std::fmt;

use serde::Deserialize;
use serde::de::{IgnoredAny, MapAccess, Visitor};
use yamalgam_serde::from_str;

/// A map visitor that asks for one value more than the mapping holds,
/// without calling `next_key_seed` first. This positions the deserializer
/// at `MappingEnd` when `skip_value` starts walking — the depth counter
/// must not underflow.
#[derive(Debug)]
struct ExtraValueRequest;

impl<'de> Deserialize<'de> for ExtraValueRequest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ExtraValueVisitor;

        impl<'de> Visitor<'de> for ExtraValueVisitor {
            type Value = ExtraValueRequest;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a mapping")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                // Consume the single real entry.
                let _key: Option<IgnoredAny> = map.next_key()?;
                let _value: IgnoredAny = map.next_value()?;
                // Contract violation: a second value request with no key.
                let _extra: IgnoredAny = map.next_value()?;
                Ok(ExtraValueRequest)
            }
        }

        deserializer.deserialize_map(ExtraValueVisitor)
    }
}

#[test]
fn extra_value_request_errors_instead_of_panicking() {
    let result: Result<ExtraValueRequest, _> = from_str("a: 1\n");
    assert!(
        result.is_err(),
        "expected error for unbalanced skip, got {result:?}"
    );
}
