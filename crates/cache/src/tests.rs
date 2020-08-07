use super::config::tests::test_prolog;
use super::*;
use std::fs;

// Since cache system is a global thing, each test needs to be run in seperate process.
// So, init() tests are run as integration tests.
// However, caching is a private thing, an implementation detail, and needs to be tested
// from the inside of the module.
// We test init() in exactly one test, rest of the tests doesn't rely on it.

#[test]
fn test_cache_init() {
    let (_tempdir, cache_dir, config_path) = test_prolog();
    let baseline_compression_level = 4;
    let config_content = format!(
        "[cache]\n\
         enabled = true\n\
         directory = {}\n\
         baseline-compression-level = {}\n",
        toml::to_string_pretty(&format!("{}", cache_dir.display())).unwrap(),
        baseline_compression_level,
    );
    fs::write(&config_path, config_content).expect("Failed to write test config file");

    let cache_config = CacheConfig::from_file(Some(&config_path)).unwrap();

    // test if we can use config
    assert!(cache_config.enabled());
    // assumption: config init creates cache directory and returns canonicalized path
    assert_eq!(
        *cache_config.directory(),
        fs::canonicalize(cache_dir).unwrap()
    );
    assert_eq!(
        cache_config.baseline_compression_level(),
        baseline_compression_level
    );

    // test if we can use worker
    cache_config.worker().on_cache_update_async(config_path);
}

#[test]
fn test_write_read_cache() {
    let (_tempdir, cache_dir, config_path) = test_prolog();
    let cache_config = load_config!(
        config_path,
        "[cache]\n\
         enabled = true\n\
         directory = {cache_dir}\n\
         baseline-compression-level = 3\n",
        cache_dir
    );
    assert!(cache_config.enabled());

    // assumption: config load creates cache directory and returns canonicalized path
    assert_eq!(
        *cache_config.directory(),
        fs::canonicalize(cache_dir).unwrap()
    );

    let compiler1 = "test-1";
    let compiler2 = "test-2";

    let entry1 = ModuleCacheEntry::from_inner(ModuleCacheEntryInner::new(compiler1, &cache_config));
    let entry2 = ModuleCacheEntry::from_inner(ModuleCacheEntryInner::new(compiler2, &cache_config));

    entry1.get_data::<_, i32, i32>(1, |_| Ok(100)).unwrap();
    entry1.get_data::<_, i32, i32>(1, |_| panic!()).unwrap();

    entry1.get_data::<_, i32, i32>(2, |_| Ok(100)).unwrap();
    entry1.get_data::<_, i32, i32>(1, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(2, |_| panic!()).unwrap();

    entry1.get_data::<_, i32, i32>(3, |_| Ok(100)).unwrap();
    entry1.get_data::<_, i32, i32>(1, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(2, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(3, |_| panic!()).unwrap();

    entry1.get_data::<_, i32, i32>(4, |_| Ok(100)).unwrap();
    entry1.get_data::<_, i32, i32>(1, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(2, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(3, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(4, |_| panic!()).unwrap();

    entry2.get_data::<_, i32, i32>(1, |_| Ok(100)).unwrap();
    entry1.get_data::<_, i32, i32>(1, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(2, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(3, |_| panic!()).unwrap();
    entry1.get_data::<_, i32, i32>(4, |_| panic!()).unwrap();
    entry2.get_data::<_, i32, i32>(1, |_| panic!()).unwrap();
}
