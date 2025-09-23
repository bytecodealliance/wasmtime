//! Measure the compilation time of files in `benches/compile`.
//!
//! Drop in new `*.wasm` or `*.wat` files in `benches/compile` to add
//! benchmarks. To try new compilation configurations, modify [`Scenario`].

use core::fmt;
use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main};
use std::path::Path;
use wasmtime::*;

/// A compilation configuration, for benchmarking.
#[derive(Clone, Copy, Debug)]
struct Scenario {
    compiler: Strategy,
    opt_level: Option<OptLevel>,
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let compiler = match self.compiler {
            Strategy::Auto => "auto",
            Strategy::Cranelift => "cranelift",
            Strategy::Winch => "winch",
            _ => unreachable!(),
        };
        let opt_level = match self.opt_level {
            Some(OptLevel::None) => "none",
            Some(OptLevel::Speed) => "speed",
            Some(OptLevel::SpeedAndSize) => "speed_and_size",
            None => "none",
            _ => unreachable!(),
        };
        write!(f, "[{compiler}:{opt_level}]")
    }
}

impl Scenario {
    fn new(compiler: Strategy, opt_level: Option<OptLevel>) -> Self {
        Scenario {
            compiler,
            opt_level,
        }
    }

    fn list() -> Vec<Self> {
        vec![
            Scenario::new(Strategy::Cranelift, Some(OptLevel::None)),
            Scenario::new(Strategy::Cranelift, Some(OptLevel::Speed)),
            Scenario::new(Strategy::Cranelift, Some(OptLevel::SpeedAndSize)),
            Scenario::new(Strategy::Winch, None),
        ]
    }

    fn to_config(&self) -> Config {
        let mut config = Config::default();
        config.strategy(self.compiler);
        if let Some(opt_level) = self.opt_level {
            config.cranelift_opt_level(opt_level);
        }
        config
    }
}

fn compile(group: &mut BenchmarkGroup<WallTime>, path: &Path, scenario: Scenario) {
    let filename = path.file_name().unwrap().to_str().unwrap();
    let id = BenchmarkId::new("compile", filename);
    let bytes = std::fs::read(path).expect("failed to read file");
    let config = scenario.to_config();
    let engine = Engine::new(&config).expect("failed to create engine");
    group.bench_function(id, |b| {
        b.iter(|| Module::new(&engine, &bytes).unwrap());
    });
}

fn bench_compile(c: &mut Criterion) {
    for scenario in Scenario::list() {
        let mut group = c.benchmark_group(scenario.to_string());
        for file in std::fs::read_dir("benches/compile").unwrap() {
            let path = file.unwrap().path();
            let extension = path.extension().and_then(|s| s.to_str());
            if path.is_file() && matches!(extension, Some("wasm") | Some("wat")) {
                compile(&mut group, &path, scenario);
            }
        }
        group.finish();
    }
}

criterion_group!(benches, bench_compile);
criterion_main!(benches);
