use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// Modes of cache operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum CacheMode {
    /// No caching. Delegates all queries to the solver.
    Off,
    /// Read-only, enforcing. Use cached results; fail the run on cache miss.
    ReadOnlyEnforcing,
    /// Read-write. Use cached results when available; invoke the solver and
    /// cache new results on miss.
    ReadWrite,
}

/// The result of a cached SMT check-sat query.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CacheVerdict {
    Success,
    Failure,
    Unknown,
    Inapplicable,
}

impl std::fmt::Display for CacheVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheVerdict::Success => write!(f, "Success"),
            CacheVerdict::Failure => write!(f, "Failure"),
            CacheVerdict::Unknown => write!(f, "Unknown"),
            CacheVerdict::Inapplicable => write!(f, "Inapplicable"),
        }
    }
}

/// A serializable representation of an SMT counterexample model, stored in
/// cache entries for failure verdicts. Uses string keys/values for simplicity
/// since the real `Model` type (`HashMap<ExprId, Const>`) is not serializable.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheModel {
    /// Mapping of expression names to their constant values in the model.
    pub values: HashMap<String, String>,
}

/// A cache entry storing an SMT query result.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheEntry {
    /// Full SHA-256 hex digest of the SMT2 text + solver backend.
    pub key_sha256: String,
    /// Short key (first 12 hex chars) used for file naming.
    pub short_key: String,
    /// SMT-LIB2 transcript that produced this result.
    pub smt2_text: String,
    /// Which solver backend was used (e.g., "cvc5" or "z3").
    pub solver_backend: String,
    /// Solver version string (e.g., "z3 4.13.3").
    #[serde(default)]
    pub solver_version: Option<String>,
    /// Verdict of the check-sat query.
    pub verdict: CacheVerdict,
    /// Counterexample model (only present for "failure" verdict).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<CacheModel>,
    /// Timing: milliseconds for encoding phase.
    pub init_ms: u64,
    /// Timing: milliseconds for the check-sat query.
    pub query_ms: u64,
}

/// Statistics accumulated during a run.
#[derive(Debug, Default)]
pub struct CacheStats {
    hits: AtomicUsize,
    misses: AtomicUsize,
    stores: AtomicUsize,
    errors: AtomicUsize,
}

impl CacheStats {
    fn inc_hits(&self) {
        self.hits.fetch_add(1, Relaxed);
    }

    fn inc_misses(&self) {
        self.misses.fetch_add(1, Relaxed);
    }

    fn inc_stores(&self) {
        self.stores.fetch_add(1, Relaxed);
    }

    #[allow(dead_code)]
    fn inc_errors(&self) {
        self.errors.fetch_add(1, Relaxed);
    }

    /// Snapshot current stats as (hits, misses, stores, errors).
    fn snapshot(&self) -> (usize, usize, usize, usize) {
        (
            self.hits.load(Relaxed),
            self.misses.load(Relaxed),
            self.stores.load(Relaxed),
            self.errors.load(Relaxed),
        )
    }
}

/// Persistent cache store backed by a directory of JSON files.
pub struct CacheStore {
    /// Directory containing cache entry files.
    dir: PathBuf,
    /// Operating mode.
    mode: CacheMode,
    /// Runtime statistics.
    stats: CacheStats,
}

impl CacheStore {
    /// Open a cache at the given directory and mode.
    ///
    /// Creates the directory if it doesn't exist.
    pub fn open(dir: PathBuf, mode: CacheMode) -> Self {
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("failed to create cache directory {}: {e}", dir.display()));
        Self {
            dir,
            mode,
            stats: CacheStats::default(),
        }
    }

    /// Compute the cache key from SMT2 text and solver backend name.
    ///
    /// The key is the SHA-256 hash of `"{solver_backend}\n{smt2_text}"`.
    /// Returns `(short_key, full_sha256)` where `short_key` is the first
    /// 12 hex characters of the digest, used for file naming.
    pub fn compute_key(smt2_text: &str, solver_backend: &str) -> (String, String) {
        let input = format!("{solver_backend}\n{smt2_text}");
        let hash = sha2::Sha256::digest(input.as_bytes());
        let full = format!("{hash:x}");
        let short = full[..12].to_string();
        (short, full)
    }

    /// Look up a cached result by key.
    ///
    /// - `Ok(Some(entry))` on cache hit (file exists and full hash matches)
    /// - `Ok(None)` on cache miss (no file, parse error, or hash mismatch)
    /// - `Err(e)` on unexpected I/O errors
    pub fn lookup(&self, short_key: &str, expected_sha256: &str) -> Result<Option<CacheEntry>> {
        let path = self.dir.join(format!("{short_key}.json"));

        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading cache entry {}", path.display()))?;

        let entry: CacheEntry = match serde_json::from_str(&contents) {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!(
                    "cache entry {} has corrupted JSON: {e} — treating as miss",
                    path.display()
                );
                return Ok(None);
            }
        };

        // Verify the full hash to rule out collisions and corruption.
        if entry.key_sha256 != expected_sha256 {
            log::warn!(
                "cache hash mismatch for key {short_key}: \
                 expected {expected_sha256}, got {}",
                entry.key_sha256
            );
            return Ok(None);
        }

        Ok(Some(entry))
    }

    /// Store a new cache entry.
    ///
    /// Uses atomic write (temp file + rename) to prevent corruption from
    /// concurrent writes. No-op in `ReadOnlyEnforcing` mode.
    pub fn store(&self, entry: CacheEntry) -> Result<()> {
        if self.mode == CacheMode::ReadOnlyEnforcing {
            return Ok(());
        }

        let path = self.dir.join(format!("{}.json", entry.short_key));
        let contents =
            serde_json::to_string_pretty(&entry).with_context(|| "serializing cache entry")?;

        // Write to a temp file first, then rename for atomicity.
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &contents)
            .with_context(|| format!("writing cache entry temp file {}", tmp_path.display()))?;

        std::fs::rename(&tmp_path, &path).with_context(|| {
            format!(
                "renaming cache entry {} -> {}",
                tmp_path.display(),
                path.display()
            )
        })?;

        self.stats.inc_stores();
        Ok(())
    }

    /// Check a query against the cache.
    ///
    /// - `Ok(Some(verdict, model))` on cache hit
    /// - `Ok(None)` on cache miss (caller should invoke solver)
    /// - `Err` on miss in `ReadOnlyEnforcing` mode
    pub fn check(
        &self,
        smt2_text: &str,
        solver_backend: &str,
    ) -> Result<Option<(CacheVerdict, Option<CacheModel>)>> {
        let (short_key, full_sha256) = Self::compute_key(smt2_text, solver_backend);

        match self.lookup(&short_key, &full_sha256)? {
            Some(entry) => {
                self.stats.inc_hits();
                Ok(Some((entry.verdict, entry.model)))
            }
            None => {
                self.stats.inc_misses();
                if self.mode == CacheMode::ReadOnlyEnforcing {
                    bail!(
                        "cache miss in read-only-enforcing mode: \
                         key {short_key} not found in {}",
                        self.dir.display()
                    );
                }
                Ok(None)
            }
        }
    }

    /// Print cache statistics summary to stdout.
    pub fn print_stats(&self) {
        let (hits, misses, stores, errors) = self.stats.snapshot();
        let total = hits + misses;
        let hit_pct = if total > 0 {
            hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let miss_pct = 100.0 - hit_pct;

        let mode_str = match self.mode {
            CacheMode::Off => unreachable!("should not print stats when cache is off"),
            CacheMode::ReadOnlyEnforcing => "read-only-enforcing",
            CacheMode::ReadWrite => "read-write",
        };

        println!("========================== Cache statistics ===========================");
        println!("Mode:           {mode_str}");
        println!("Directory:      {}", self.dir.display());
        println!("Hits:           {hits} ({hit_pct:.1}%)");
        println!("Misses:         {misses} ({miss_pct:.1}%)");
        println!("New entries:    {stores}");
        println!("Errors:         {errors}");
        println!("========================================================================");
    }

    /// Get a snapshot of current stats as (hits, misses, stores, errors).
    pub fn snapshot_stats(&self) -> (usize, usize, usize, usize) {
        self.stats.snapshot()
    }

    /// Get the current cache mode.
    pub fn mode(&self) -> CacheMode {
        self.mode
    }

    /// Get the cache directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn temp_cache_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("veri_cache_test_{}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_roundtrip() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(dir.clone(), CacheMode::ReadWrite);
        let dir_for_cleanup = dir.clone();

        let (short_key, full_sha256) = CacheStore::compute_key("(set-logic ALL)", "cvc5");
        let entry = CacheEntry {
            key_sha256: full_sha256.clone(),
            short_key: short_key.clone(),
            smt2_text: "(set-logic ALL)".to_string(),
            solver_backend: "cvc5".to_string(),
            solver_version: Some("cvc5 1.2.0".to_string()),
            verdict: CacheVerdict::Success,
            model: None,
            init_ms: 5,
            query_ms: 10,
        };

        store.store(entry).expect("store should succeed");

        let looked_up = store
            .lookup(&short_key, &full_sha256)
            .expect("lookup should succeed");
        assert!(looked_up.is_some());
        let found = looked_up.unwrap();
        assert_eq!(found.verdict, CacheVerdict::Success);
        assert_eq!(found.solver_backend, "cvc5");
        assert_eq!(found.solver_version, Some("cvc5 1.2.0".to_string()));
        assert_eq!(found.init_ms, 5);
        assert_eq!(found.query_ms, 10);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_missing_key() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(dir, CacheMode::ReadWrite);

        let result = store
            .lookup("nonexistent", "00000000000000000000000000000000")
            .expect("lookup should succeed");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_hit() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(dir.clone(), CacheMode::ReadWrite);

        // Store an entry
        let smt2_text = "(set-logic ALL)";
        let solver = "cvc5";
        let (short_key, full_sha256) = CacheStore::compute_key(smt2_text, solver);
        let entry = CacheEntry {
            key_sha256: full_sha256,
            short_key,
            smt2_text: smt2_text.to_string(),
            solver_backend: solver.to_string(),
            solver_version: None,
            verdict: CacheVerdict::Failure,
            model: Some(CacheModel {
                values: HashMap::from([("x".to_string(), "42".to_string())]),
            }),
            init_ms: 1,
            query_ms: 2,
        };
        store.store(entry).expect("store should succeed");

        // Check should return the cached result
        let result = store
            .check(smt2_text, solver)
            .expect("check should succeed");
        assert!(result.is_some());
        let (verdict, model) = result.unwrap();
        assert_eq!(verdict, CacheVerdict::Failure);
        assert!(model.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_check_miss_readwrite() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(dir, CacheMode::ReadWrite);

        let result = store.check("(unknown query)", "cvc5");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let (hits, misses, stores, errors) = store.snapshot_stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 1);
        assert_eq!(stores, 0);
        assert_eq!(errors, 0);
    }

    #[test]
    fn test_check_miss_readonly_enforcing() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(dir.clone(), CacheMode::ReadOnlyEnforcing);

        let result = store.check("(unknown query)", "cvc5");
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
