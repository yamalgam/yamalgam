//! Native yg filter registration.

use jaq_core::RunPtr;
use jaq_core::native::{Filter, Fun, bome, run, v};

use crate::engine::YgKind;
use crate::value::YamlVal;

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
    Box::new([("_yaml_type", v(0), |context_value| {
        bome(Ok(YamlVal::str(context_value.1.type_name())))
    })])
}
