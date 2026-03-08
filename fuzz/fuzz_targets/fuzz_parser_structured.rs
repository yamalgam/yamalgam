#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_fuzz::YamlDoc;
use yamalgam_parser::Parser;

fuzz_target!(|doc: YamlDoc| {
    let yaml = doc.render();
    let _ = Parser::new(&yaml).collect::<Result<Vec<_>, _>>();
});
