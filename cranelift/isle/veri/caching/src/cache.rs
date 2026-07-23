//! Persistent store of SMT query responses.

use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use serde::{Deserialize, Serialize};
use sha2::Digest;

/// Modes of cache operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl std::str::FromStr for CacheMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read-only-enforcing" => Ok(CacheMode::ReadOnlyEnforcing),
            "read-write" => Ok(CacheMode::ReadWrite),
            _ => Err(format!(
                "unknown cache mode '{s}' (expected 'read-only-enforcing' or 'read-write')"
            )),
        }
    }
}

impl std::fmt::Display for CacheMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CacheMode::ReadOnlyEnforcing => "read-only-enforcing",
            CacheMode::ReadWrite => "read-write",
        })
    }
}

/// A cache entry storing the response to a single SMT query.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CacheEntry {
    /// Full SHA-256 hex digest of the solver name + replay script + query.
    key_sha256: String,
    /// Which solver was used (e.g., "cvc5" or "z3").
    solver: String,
    /// The solver's response, as a JSON s-expression tree: atoms are strings,
    /// lists are arrays (see [`crate::convert`]).
    response: serde_json::Value,
}

impl CacheEntry {
    fn short_key(&self) -> &str {
        &self.key_sha256[..12]
    }
}

/// Statistics accumulated during a run.
#[derive(Debug, Default)]
struct CacheStats {
    hits: AtomicUsize,
    misses: AtomicUsize,
    stores: AtomicUsize,
    retained: AtomicUsize,
}

/// Persistent cache store backed by directories of JSON files.
///
/// A cache has an optional read-only `source` directory and an optional
/// read-write `destination` directory. Lookups consult the destination first,
/// then the source. In [`CacheMode::ReadWrite`], a result found only in the
/// source is copied into the destination, so that after a full run the
/// destination contains exactly the entries the run used — the basis for a
/// garbage-collecting cache rebuild.
pub struct Cache {
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

impl Cache {
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

    /// The mode this cache operates in.
    pub fn mode(&self) -> CacheMode {
        self.mode
    }

    /// Compute the cache key for a query.
    ///
    /// The key is the SHA-256 hash of `"{solver}\n{script}"`, where `script`
    /// is the replay script of the path so far followed by the query command.
    fn compute_key(solver: &str, script: &str) -> String {
        let mut hash = sha2::Sha256::new();
        hash.update(solver.as_bytes());
        hash.update(b"\n");
        hash.update(script.as_bytes());
        format!("{:x}", hash.finalize())
    }

    /// Look up a cached entry by key in a specific directory.
    ///
    /// - `Ok(Some(entry))` on cache hit (file exists and full hash matches)
    /// - `Ok(None)` on cache miss (no file, parse error, or hash mismatch)
    /// - `Err(e)` on unexpected I/O errors
    fn lookup_in(dir: &Path, expected_sha256: &str) -> io::Result<Option<CacheEntry>> {
        let short_key = &expected_sha256[..12];
        let path = dir.join(format!("{short_key}.json"));

        let contents = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };

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
    fn write_entry(dir: &Path, entry: &CacheEntry) -> io::Result<()> {
        let path = dir.join(format!("{}.json", entry.short_key()));
        let contents = serde_json::to_string_pretty(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write to a temp file first, then rename for atomicity. The temp file
        // name is made unique per write so concurrent writers (including two
        // threads racing to persist the same key) don't clobber each other's
        // temp file.
        static TMP_SEQ: AtomicUsize = AtomicUsize::new(0);
        let seq = TMP_SEQ.fetch_add(1, Relaxed);
        let tmp_path = path.with_extension(format!("{}.{seq}.tmp", std::process::id()));
        std::fs::write(&tmp_path, &contents)?;
        std::fs::rename(&tmp_path, &path)?;

        Ok(())
    }

    /// Look up the cached response for a query against `solver` whose full
    /// replay script (path plus query command) is `script`.
    ///
    /// Consults the destination cache first, then the source. In read-write
    /// mode, a result found only in the source is copied into the destination
    /// (so the destination accumulates exactly the entries the run used).
    ///
    /// Returns the response on a hit, `None` on a miss. A miss in
    /// [`CacheMode::ReadOnlyEnforcing`] is reported by the caller, which has
    /// the context to produce a useful error.
    pub(crate) fn lookup(
        &self,
        solver: &str,
        script: &str,
    ) -> io::Result<Option<serde_json::Value>> {
        let key = Self::compute_key(solver, script);

        // Destination first: freshly-written or already-retained entries.
        if let Some(dest) = &self.dest
            && let Some(entry) = Self::lookup_in(dest, &key)?
        {
            self.stats.hits.fetch_add(1, Relaxed);
            return Ok(Some(entry.response));
        }

        // Then the read-only source. Retain any hit into the destination so
        // that the destination ends up holding exactly the used entries.
        if let Some(source) = &self.source
            && let Some(entry) = Self::lookup_in(source, &key)?
        {
            self.stats.hits.fetch_add(1, Relaxed);
            if let Some(dest) = &self.dest {
                Self::write_entry(dest, &entry)?;
                self.stats.retained.fetch_add(1, Relaxed);
            }
            return Ok(Some(entry.response));
        }

        self.stats.misses.fetch_add(1, Relaxed);
        Ok(None)
    }

    /// Store a freshly-computed response into the destination cache.
    ///
    /// No-op if there is no destination directory.
    pub(crate) fn store(
        &self,
        solver: &str,
        script: &str,
        response: &serde_json::Value,
    ) -> io::Result<()> {
        let Some(dest) = &self.dest else {
            return Ok(());
        };
        let key = Self::compute_key(solver, script);
        let entry = CacheEntry {
            key_sha256: key,
            solver: solver.to_string(),
            response: response.clone(),
        };
        Self::write_entry(dest, &entry)?;
        self.stats.stores.fetch_add(1, Relaxed);
        Ok(())
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
        let (hits, misses, stores, retained) = self.snapshot_stats();
        let total = hits + misses;
        let hit_pct = if total > 0 {
            hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let miss_pct = 100.0 - hit_pct;

        let dir_str = |d: &Option<PathBuf>| match d {
            Some(p) => p.display().to_string(),
            None => "(none)".to_string(),
        };

        println!("========================== Cache statistics ===========================");
        println!("Mode:            {}", self.mode);
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

    /// Get a snapshot of current stats as (hits, misses, stores, retained).
    pub fn snapshot_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.stats.hits.load(Relaxed),
            self.stats.misses.load(Relaxed),
            self.stats.stores.load(Relaxed),
            self.stats.retained.load(Relaxed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(None, Some(dir.path().to_path_buf()), CacheMode::ReadWrite);

        cache
            .store("cvc5", "(set-logic ALL)\n(check-sat)", &json!("unsat"))
            .expect("store should succeed");

        let result = cache
            .lookup("cvc5", "(set-logic ALL)\n(check-sat)")
            .expect("lookup should succeed");
        assert_eq!(result, Some(json!("unsat")));

        // A different path is a different key.
        let result = cache
            .lookup("cvc5", "(set-logic QF_BV)\n(check-sat)")
            .expect("lookup should succeed");
        assert_eq!(result, None);

        // A different solver is a different key.
        let result = cache
            .lookup("z3", "(set-logic ALL)\n(check-sat)")
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn test_retain_from_source() {
        let source = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();

        // Populate a source cache with one entry.
        {
            let seed = Cache::open(
                None,
                Some(source.path().to_path_buf()),
                CacheMode::ReadWrite,
            );
            seed.store("cvc5", "(check-sat)", &json!("sat"))
                .expect("seed store should succeed");
        }

        // Read-write with a fresh destination: a source hit is retained into
        // the destination (garbage-collecting rebuild).
        let cache = Cache::open(
            Some(source.path().to_path_buf()),
            Some(dest.path().to_path_buf()),
            CacheMode::ReadWrite,
        );
        let result = cache
            .lookup("cvc5", "(check-sat)")
            .expect("lookup should succeed");
        assert_eq!(result, Some(json!("sat")));

        let (hits, misses, stores, retained) = cache.snapshot_stats();
        assert_eq!((hits, misses, stores, retained), (1, 0, 0, 1));

        // The entry now exists in the destination too.
        assert_eq!(Cache::count_entries(dest.path()), 1);
    }
}
