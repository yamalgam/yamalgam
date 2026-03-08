#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_core::LoaderConfig;
use yamalgam_parser::Parser;
use yamalgam_scanner::scanner::Scanner;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let config = LoaderConfig::strict();
        let _ = Scanner::with_config(input, &config).collect::<Result<Vec<_>, _>>();
        let _ = Parser::with_config(input, &config).collect::<Result<Vec<_>, _>>();
    }
});
