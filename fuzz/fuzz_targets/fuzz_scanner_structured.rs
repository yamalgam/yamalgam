#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_fuzz::YamlDoc;
use yamalgam_scanner::scanner::Scanner;

fuzz_target!(|doc: YamlDoc| {
    let yaml = doc.render();
    let _ = Scanner::new(&yaml).collect::<Result<Vec<_>, _>>();
});
