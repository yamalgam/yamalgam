#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_scanner::scanner::Scanner;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    if input.is_empty() {
        return;
    }

    // Yamalgam scanner
    let rust_result = Scanner::new(input).collect::<Result<Vec<_>, _>>();

    // Differential comparison against C harness requires subprocess.
    // For now, just verify yamalgam doesn't panic on any input.
    // Full differential fuzzing will be enabled when the C harness
    // is available in CI.
    drop(rust_result);
});
