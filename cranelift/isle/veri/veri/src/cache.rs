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
    /// Read-only, enforcing. Use cached results from the source cache; fail the
    /// run on a cache miss. Never invokes the solver.
    ReadOnlyEnforcing,
    /// Read-write. Use cached results found in either the source or the
    /// destination cache; on a miss, invoke the solver and write the new
    /// result to the destination. Results found only in the source are also
    /// copied into the destination, so the destination ends up holding exactly
    /// the entries the run used — enabling cache garbage collection (a rebuild
    /// that drops unused entries).
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
    retained: AtomicUsize,
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

    fn inc_retained(&self) {
        self.retained.fetch_add(1, Relaxed);
    }

    #[allow(dead_code)]
    fn inc_errors(&self) {
        self.errors.fetch_add(1, Relaxed);
    }

    /// Snapshot current stats as (hits, misses, stores, retained, errors).
    fn snapshot(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.hits.load(Relaxed),
            self.misses.load(Relaxed),
            self.stores.load(Relaxed),
            self.retained.load(Relaxed),
            self.errors.load(Relaxed),
        )
    }
}

/// Persistent cache store backed by directories of JSON files.
///
/// A store has an optional read-only `source` directory and an optional
/// read-write `destination` directory. Lookups consult the destination first,
/// then the source. In [`CacheMode::ReadWrite`], a result found only in the
/// source is copied into the destination, so that after a full run the
/// destination contains exactly the entries the run used — the basis for a
/// garbage-collecting cache rebuild.
pub struct CacheStore {
    /// Read-only source directory consulted for cache hits.
    source: Option<PathBuf>,
    /// Destination directory: consulted for hits, and where entries are
    /// written — both freshly-computed results and hits retained from
    /// `source`.
    dest: Option<PathBuf>,
    /// Operating mode.
    mode: CacheMode,
    /// Runtime statistics.
    stats: CacheStats,
}

impl CacheStore {
    /// Open a cache with the given source and destination directories and mode.
    ///
    /// The destination directory (if any) is created if it doesn't exist. The
    /// source directory is treated as read-only and is not created.
    pub fn open(source: Option<PathBuf>, dest: Option<PathBuf>, mode: CacheMode) -> Self {
        if let Some(dest) = &dest {
            std::fs::create_dir_all(dest).unwrap_or_else(|e| {
                panic!("failed to create cache directory {}: {e}", dest.display())
            });
        }
        Self {
            source,
            dest,
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

    /// Look up a cached result by key in a specific directory.
    ///
    /// - `Ok(Some(entry))` on cache hit (file exists and full hash matches)
    /// - `Ok(None)` on cache miss (no file, parse error, or hash mismatch)
    /// - `Err(e)` on unexpected I/O errors
    fn lookup_in(dir: &Path, short_key: &str, expected_sha256: &str) -> Result<Option<CacheEntry>> {
        let path = dir.join(format!("{short_key}.json"));

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

    /// Write a cache entry into `dir`, using an atomic temp-file + rename to
    /// prevent corruption from concurrent writes.
    fn write_entry(dir: &Path, entry: &CacheEntry) -> Result<()> {
        let path = dir.join(format!("{}.json", entry.short_key));
        let contents =
            serde_json::to_string_pretty(entry).with_context(|| "serializing cache entry")?;

        // Write to a temp file first, then rename for atomicity. The temp file
        // name is made unique per write so concurrent writers (including two
        // threads racing to persist the same key) don't clobber each other's
        // temp file.
        static TMP_SEQ: AtomicUsize = AtomicUsize::new(0);
        let seq = TMP_SEQ.fetch_add(1, Relaxed);
        let tmp_path = path.with_extension(format!("{}.{seq}.tmp", std::process::id()));
        std::fs::write(&tmp_path, &contents)
            .with_context(|| format!("writing cache entry temp file {}", tmp_path.display()))?;

        std::fs::rename(&tmp_path, &path).with_context(|| {
            format!(
                "renaming cache entry {} -> {}",
                tmp_path.display(),
                path.display()
            )
        })?;

        Ok(())
    }

    /// Store a freshly-computed cache entry into the destination cache.
    ///
    /// No-op if there is no destination directory (e.g. read-only-enforcing).
    pub fn store(&self, entry: CacheEntry) -> Result<()> {
        let Some(dest) = &self.dest else {
            return Ok(());
        };
        Self::write_entry(dest, &entry)?;
        self.stats.inc_stores();
        Ok(())
    }

    /// Check a query against the cache.
    ///
    /// Consults the destination cache first, then the source. In read-write
    /// mode, a result found only in the source is copied into the destination
    /// (so the destination accumulates exactly the entries the run used).
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

        // Destination first: freshly-written or already-retained entries.
        if let Some(dest) = &self.dest
            && let Some(entry) = Self::lookup_in(dest, &short_key, &full_sha256)?
        {
            self.stats.inc_hits();
            return Ok(Some((entry.verdict, entry.model)));
        }

        // Then the read-only source. Retain any hit into the destination so
        // that the destination ends up holding exactly the used entries.
        if let Some(source) = &self.source
            && let Some(entry) = Self::lookup_in(source, &short_key, &full_sha256)?
        {
            self.stats.inc_hits();
            if let Some(dest) = &self.dest {
                Self::write_entry(dest, &entry)?;
                self.stats.inc_retained();
            }
            return Ok(Some((entry.verdict, entry.model)));
        }

        // Miss.
        self.stats.inc_misses();
        if self.mode == CacheMode::ReadOnlyEnforcing {
            bail!("cache miss in read-only-enforcing mode: key {short_key} not found");
        }
        Ok(None)
    }

    /// Count the number of cache entry files (`*.json`) in a directory.
    fn count_entries(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                    .count()
            })
            .unwrap_or(0)
    }

    /// Print cache statistics summary to stdout.
    pub fn print_stats(&self) {
        let (hits, misses, stores, retained, _errors) = self.stats.snapshot();
        let total = hits + misses;
        let hit_pct = if total > 0 {
            hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let miss_pct = 100.0 - hit_pct;

        let mode_str = match self.mode {
            CacheMode::ReadOnlyEnforcing => "read-only-enforcing",
            CacheMode::ReadWrite => "read-write",
        };
        let dir_str = |d: &Option<PathBuf>| match d {
            Some(p) => p.display().to_string(),
            None => "(none)".to_string(),
        };

        println!("========================== Cache statistics ===========================");
        println!("Mode:            {mode_str}");
        println!("Source:          {}", dir_str(&self.source));
        println!("Destination:     {}", dir_str(&self.dest));
        println!("Hits:            {hits} ({hit_pct:.1}%)");
        println!("Misses:          {misses} ({miss_pct:.1}%)");
        println!("New entries:     {stores}");
        println!("Retained:        {retained}");

        // When rebuilding into a fresh destination, report how many source
        // entries went unused (and are therefore dropped by the rebuild).
        if let (Some(source), Some(dest)) = (&self.source, &self.dest)
            && source != dest
        {
            let source_count = Self::count_entries(source);
            let dropped = source_count.saturating_sub(retained);
            println!("Source entries:  {source_count}");
            println!("Dropped (unused):{dropped:>4}");
        }
        println!("========================================================================");
    }

    /// Get a snapshot of current stats as (hits, misses, stores, retained,
    /// errors).
    pub fn snapshot_stats(&self) -> (usize, usize, usize, usize, usize) {
        self.stats.snapshot()
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

    fn entry(smt2_text: &str, solver: &str, verdict: CacheVerdict) -> CacheEntry {
        let (short_key, full_sha256) = CacheStore::compute_key(smt2_text, solver);
        CacheEntry {
            key_sha256: full_sha256,
            short_key,
            smt2_text: smt2_text.to_string(),
            solver_backend: solver.to_string(),
            solver_version: None,
            verdict,
            model: None,
            init_ms: 5,
            query_ms: 10,
        }
    }

    #[test]
    fn test_roundtrip() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(None, Some(dir.clone()), CacheMode::ReadWrite);

        store
            .store(entry("(set-logic ALL)", "cvc5", CacheVerdict::Success))
            .expect("store should succeed");

        let result = store
            .check("(set-logic ALL)", "cvc5")
            .expect("check should succeed");
        assert_eq!(result.map(|(v, _)| v), Some(CacheVerdict::Success));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_check_hit_with_model() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(None, Some(dir.clone()), CacheMode::ReadWrite);

        let mut e = entry("(set-logic ALL)", "cvc5", CacheVerdict::Failure);
        e.model = Some(CacheModel {
            values: HashMap::from([("x".to_string(), "42".to_string())]),
        });
        store.store(e).expect("store should succeed");

        let (verdict, model) = store
            .check("(set-logic ALL)", "cvc5")
            .expect("check should succeed")
            .expect("should be a hit");
        assert_eq!(verdict, CacheVerdict::Failure);
        assert!(model.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_check_miss_readwrite() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(None, Some(dir), CacheMode::ReadWrite);

        let result = store.check("(unknown query)", "cvc5");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let (hits, misses, stores, retained, _errors) = store.snapshot_stats();
        assert_eq!((hits, misses, stores, retained), (0, 1, 0, 0));
    }

    #[test]
    fn test_check_miss_readonly_enforcing() {
        let dir = temp_cache_dir();
        let store = CacheStore::open(Some(dir), None, CacheMode::ReadOnlyEnforcing);

        let result = store.check("(unknown query)", "cvc5");
        assert!(result.is_err());
    }

    #[test]
    fn test_retain_from_source() {
        // Populate a source cache with one entry.
        let source = temp_cache_dir();
        let dest = temp_cache_dir();
        {
            let seed = CacheStore::open(None, Some(source.clone()), CacheMode::ReadWrite);
            seed.store(entry("(set-logic ALL)", "cvc5", CacheVerdict::Success))
                .expect("seed store should succeed");
        }

        // Read-write with a fresh destination: a source hit is retained into
        // the destination (garbage-collecting rebuild).
        let store = CacheStore::open(Some(source.clone()), Some(dest.clone()), CacheMode::ReadWrite);
        let result = store
            .check("(set-logic ALL)", "cvc5")
            .expect("check should succeed");
        assert_eq!(result.map(|(v, _)| v), Some(CacheVerdict::Success));

        let (hits, misses, stores, retained, _errors) = store.snapshot_stats();
        assert_eq!((hits, misses, stores, retained), (1, 0, 0, 1));

        // The entry now exists in the destination too.
        assert_eq!(CacheStore::count_entries(&dest), 1);

        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&dest);
    }
}
