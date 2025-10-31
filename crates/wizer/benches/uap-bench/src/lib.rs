use regex::RegexSet;
use serde::Deserialize;
use std::sync::LazyLock;

static UA_REGEX_SET: LazyLock<RegexSet> = LazyLock::new(|| {
    let uap_yaml = include_str!(concat!(env!("OUT_DIR"), "/regexes.yaml"));
    let parsers: UserAgentParsers = serde_yaml::from_str(uap_yaml).unwrap();
    RegexSet::new(
        parsers
            .user_agent_parsers
            .iter()
            .map(|e| e.regex.replace("\\/", "/").replace("\\!", "!")),
    )
    .unwrap()
});

#[derive(Deserialize)]
struct UserAgentParsers {
    user_agent_parsers: Vec<UserAgentParserEntry>,
}

#[derive(Deserialize)]
struct UserAgentParserEntry {
    regex: String,
    // family_replacement: Option<String>,
    // brand_replacement: Option<String>,
    // model_replacement: Option<String>,
    // os_replacement: Option<String>,
    // v1_replacement: Option<String>,
    // v2_replacement: Option<String>,
    // os_v1_replacement: Option<String>,
    // os_v2_replacement: Option<String>,
    // os_v3_replacement: Option<String>,
}

#[unsafe(export_name = "wizer-initialize")]
pub extern "C" fn init() {
    LazyLock::force(&UA_REGEX_SET);
}

#[unsafe(export_name = "run")]
pub extern "C" fn run(ptr: *mut u8, len: usize) -> i32 {
    let s = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8(slice).unwrap()
    };
    UA_REGEX_SET.is_match(&s) as u8 as i32
}

#[unsafe(export_name = "alloc")]
pub extern "C" fn alloc(size: usize, align: usize) -> *mut u8 {
    let layout = std::alloc::Layout::from_size_align(size, align).unwrap();
    unsafe { std::alloc::alloc(layout) }
}

#[unsafe(export_name = "dealloc")]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize, align: usize) {
    let layout = std::alloc::Layout::from_size_align(size, align).unwrap();
    unsafe {
        std::alloc::dealloc(ptr, layout);
    }
}
