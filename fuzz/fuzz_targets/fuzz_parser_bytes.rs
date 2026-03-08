#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = Parser::new(input).collect::<Result<Vec<_>, _>>();
    }
});
