//! Persistent store of SMT query responses.
//!
//! The cache is kept in an [`ObjectStore`] (in practice an S3 bucket) as a
//! small log-structured merge tree of immutable [`Pack`]s:
//!
//! ```text
//! base/<stamp>.pack    full snapshot; records which deltas it covers
//! delta/<stamp>.pack   the new entries computed by one run
//! ```
//!
//! where `<stamp>` is a zero-padded millisecond timestamp plus a random
//! suffix, so names sort by creation time.
//!
//! - **Open** lists the store, fetches the newest base, and applies every
//!   delta that base does not cover. The whole cache is held in memory.
//! - **Flush** (in [`CacheMode::ReadWrite`]) uploads one delta holding the
//!   entries computed during the run, if any, and appends the keys the run
//!   used (hit or computed) to a local used-keys file. One verification may
//!   be split across several runs (e.g. one process per configuration) that
//!   share the file.
//! - **Compaction** is a separate step, run after all the runs of a
//!   verification have flushed. If the number of uncovered deltas has
//!   reached a threshold, it writes a new base holding only the entries in
//!   the used-keys file, garbage-collecting the rest, and deletes the base
//!   and deltas it supersedes.
//!
//! No locking is needed because entries are deterministic functions of their
//! keys and the cache is only an optimization: concurrent writers can at
//! worst produce duplicate entries or a redundant base, and a reader racing
//! a compaction retries its load. An entry lost in any such race costs only
//! a recomputation.

use std::{
    collections::{HashMap, HashSet},
    io::{self, Write as _},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering::Relaxed},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::Digest;

use crate::pack::{Key, Pack};
use crate::store::ObjectStore;

/// Modes of cache operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// Read-only, enforcing. Use cached results; fail the run on a cache miss.
    /// Never invokes the solver.
    ReadOnlyEnforcing,
    /// Read-only. Use cached results; on a miss, invoke the solver, but never
    /// write the result back to the store.
    ReadOnly,
    /// Read-write. Use cached results; on a miss, invoke the solver, and
    /// upload the new results when the cache is flushed.
    ReadWrite,
}

impl std::str::FromStr for CacheMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read-only-enforcing" => Ok(CacheMode::ReadOnlyEnforcing),
            "read-only" => Ok(CacheMode::ReadOnly),
            "read-write" => Ok(CacheMode::ReadWrite),
            _ => Err(format!(
                "unknown cache mode '{s}' (expected 'read-only-enforcing', \
                 'read-only' or 'read-write')"
            )),
        }
    }
}

impl std::fmt::Display for CacheMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CacheMode::ReadOnlyEnforcing => "read-only-enforcing",
            CacheMode::ReadOnly => "read-only",
            CacheMode::ReadWrite => "read-write",
        })
    }
}

const BASE_DIR: &str = "base/";
const DELTA_DIR: &str = "delta/";

/// How many times to retry loading if objects vanish mid-load (because a
/// concurrent compaction deleted them).
const LOAD_ATTEMPTS: usize = 3;

/// What was loaded from the store at open time.
#[derive(Debug, Default)]
struct Snapshot {
    /// Name of the base pack loaded, if any.
    base: Option<String>,
    /// Deltas covered by the base that were still present in the listing
    /// (i.e. not yet deleted by the compaction that wrote the base).
    stale_covered: Vec<String>,
    /// Deltas applied on top of the base.
    deltas: Vec<String>,
}

/// Statistics accumulated during a run.
#[derive(Debug, Default)]
struct CacheStats {
    hits: AtomicUsize,
    misses: AtomicUsize,
    stores: AtomicUsize,
}

/// Persistent cache of SMT query responses, backed by an [`ObjectStore`].
pub struct Cache {
    store: Arc<dyn ObjectStore>,
    mode: CacheMode,
    snapshot: Snapshot,
    /// Entries loaded from the store. Immutable after open.
    loaded: HashMap<Key, String>,
    /// Entries computed during this run, to be uploaded on flush.
    new: Mutex<HashMap<Key, String>>,
    /// Keys used (hit or computed) during this run. Only tracked in
    /// [`CacheMode::ReadWrite`].
    used: Mutex<HashSet<Key>>,
    stats: CacheStats,
}

/// Summary of what a compaction wrote.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CompactionSummary {
    /// Name of the new base.
    pub base: String,
    /// Number of entries in the new base.
    pub kept: usize,
    /// Number of entries garbage-collected.
    pub dropped: usize,
    /// Number of superseded objects deleted.
    pub deleted: usize,
}

impl Cache {
    /// Open the cache in `store`, loading its full contents into memory.
    pub fn open(store: Arc<dyn ObjectStore>, mode: CacheMode) -> io::Result<Self> {
        let mut last_err = None;
        for _ in 0..LOAD_ATTEMPTS {
            match Self::load(&*store)? {
                Ok((snapshot, loaded)) => {
                    return Ok(Self {
                        store,
                        mode,
                        snapshot,
                        loaded,
                        new: Mutex::new(HashMap::new()),
                        used: Mutex::new(HashSet::new()),
                        stats: CacheStats::default(),
                    });
                }
                Err(missing) => {
                    log::warn!("cache object {missing} vanished during load; retrying");
                    last_err = Some(missing);
                }
            }
        }
        Err(io::Error::other(format!(
            "cache objects kept vanishing during load (last: {})",
            last_err.unwrap()
        )))
    }

    /// One attempt at loading the cache. The inner `Err` names an object that
    /// was listed but had disappeared by the time it was fetched.
    fn load(
        store: &dyn ObjectStore,
    ) -> io::Result<Result<(Snapshot, HashMap<Key, String>), String>> {
        let mut bases = store.list(BASE_DIR)?;
        let mut deltas = store.list(DELTA_DIR)?;
        bases.sort();
        deltas.sort();

        let mut snapshot = Snapshot::default();
        let mut loaded = HashMap::new();
        let mut covered = HashSet::new();

        if let Some(base) = bases.last() {
            let Some(bytes) = store.get(base)? else {
                return Ok(Err(base.clone()));
            };
            match Pack::decode(&bytes) {
                Ok(pack) => {
                    covered.extend(pack.covered);
                    loaded.extend(pack.entries);
                    snapshot.base = Some(base.clone());
                }
                Err(e) => log::warn!("ignoring corrupt cache base {base}: {e}"),
            }
        }

        for delta in deltas {
            if covered.contains(&delta) {
                snapshot.stale_covered.push(delta);
                continue;
            }
            let Some(bytes) = store.get(&delta)? else {
                return Ok(Err(delta));
            };
            match Pack::decode(&bytes) {
                Ok(pack) => {
                    loaded.extend(pack.entries);
                    snapshot.deltas.push(delta);
                }
                Err(e) => log::warn!("ignoring corrupt cache delta {delta}: {e}"),
            }
        }

        Ok(Ok((snapshot, loaded)))
    }

    /// The mode this cache operates in.
    pub fn mode(&self) -> CacheMode {
        self.mode
    }

    /// Compute the cache key for a query.
    ///
    /// The key is the SHA-256 hash of `"{solver}\n{script}"`, where `script`
    /// is the replay script of the path so far followed by the query command.
    fn compute_key(solver: &str, script: &str) -> Key {
        // This allows us to invalidate the whole cache (in CI and
        // locally for users) if we change something material about the
        // settings, like the timeout or a solver version.
        static CACHE_VERSION: &[u8] = b"1\n";

        let mut hash = sha2::Sha256::new();
        hash.update(solver.as_bytes());
        hash.update(b"\n");
        hash.update(script.as_bytes());
        hash.update(b"\n");
        hash.update(CACHE_VERSION);
        hash.finalize().into()
    }

    /// Look up the cached response for a query against `solver` whose full
    /// replay script (path plus query command) is `script`.
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
        let found = match self.loaded.get(&key) {
            Some(value) => Some(value.clone()),
            None => self.new.lock().unwrap().get(&key).cloned(),
        };
        let Some(value) = found else {
            self.stats.misses.fetch_add(1, Relaxed);
            return Ok(None);
        };
        match serde_json::from_str(&value) {
            Ok(response) => {
                self.stats.hits.fetch_add(1, Relaxed);
                self.mark_used(key);
                Ok(Some(response))
            }
            Err(e) => {
                log::warn!(
                    "cache entry {} has corrupted JSON: {e} — treating as miss",
                    crate::s3::hex(&key)
                );
                self.stats.misses.fetch_add(1, Relaxed);
                Ok(None)
            }
        }
    }

    /// Record a freshly-computed response. It is served to later lookups in
    /// this run and, in [`CacheMode::ReadWrite`], uploaded on [`Cache::flush`].
    pub(crate) fn store(
        &self,
        solver: &str,
        script: &str,
        response: &serde_json::Value,
    ) -> io::Result<()> {
        let key = Self::compute_key(solver, script);
        self.new.lock().unwrap().insert(key, response.to_string());
        self.mark_used(key);
        self.stats.stores.fetch_add(1, Relaxed);
        Ok(())
    }

    fn mark_used(&self, key: Key) {
        if self.mode == CacheMode::ReadWrite {
            self.used.lock().unwrap().insert(key);
        }
    }

    /// Write this run's new entries back to the store as a delta, returning
    /// its name if one was written, and append the keys this run used to
    /// `used_keys_file` (created if missing) for a later compaction. A no-op
    /// unless in [`CacheMode::ReadWrite`].
    pub fn flush(&self, used_keys_file: Option<&Path>) -> io::Result<Option<String>> {
        if self.mode != CacheMode::ReadWrite {
            return Ok(None);
        }
        let new = std::mem::take(&mut *self.new.lock().unwrap());
        let used = std::mem::take(&mut *self.used.lock().unwrap());

        if let Some(path) = used_keys_file {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            let bytes: Vec<u8> = used.iter().flatten().copied().collect();
            file.write_all(&bytes)?;
        }

        if new.is_empty() {
            return Ok(None);
        }
        let name = format!("{DELTA_DIR}{}.pack", stamp());
        let pack = Pack {
            covered: vec![],
            entries: new.into_iter().collect(),
        };
        self.store.put(&name, &pack.encode())?;
        Ok(Some(name))
    }

    /// Number of deltas not covered by the base.
    pub fn uncovered_deltas(&self) -> usize {
        self.snapshot.deltas.len()
    }

    /// If at least `threshold` deltas are uncovered, compact the cache,
    /// keeping only the entries whose keys are listed in `used_keys_file`
    /// (as written by [`Cache::flush`]). Returns `None` if below the
    /// threshold. Requires [`CacheMode::ReadWrite`].
    ///
    /// The file must list the keys used by a complete verification: anything
    /// not in it is dropped. An empty or missing file is an error, since it
    /// would drop everything.
    pub fn compact_with_used_keys_file(
        &self,
        used_keys_file: &Path,
        threshold: usize,
    ) -> io::Result<Option<CompactionSummary>> {
        if self.uncovered_deltas() < threshold.max(1) {
            return Ok(None);
        }
        let bytes = std::fs::read(used_keys_file)?;
        if bytes.is_empty() {
            return Err(io::Error::other(format!(
                "used-keys file {} is empty; refusing to compact",
                used_keys_file.display()
            )));
        }
        let used = bytes
            .chunks_exact(32)
            .map(|k| Key::try_from(k).unwrap())
            .collect();
        self.compact(&used).map(Some)
    }

    /// Write a new base holding the entries whose keys are in `used`,
    /// covering every delta we loaded, and delete what it supersedes.
    fn compact(&self, used: &HashSet<Key>) -> io::Result<CompactionSummary> {
        if self.mode != CacheMode::ReadWrite {
            return Err(io::Error::other("compaction requires read-write mode"));
        }
        let mut summary = CompactionSummary::default();

        // Cover stale deltas too, in case their deletion failed before.
        let mut covered = self.snapshot.stale_covered.clone();
        covered.extend(self.snapshot.deltas.iter().cloned());
        let mut pack = Pack {
            covered: covered.clone(),
            entries: Default::default(),
        };
        for (key, value) in &self.loaded {
            if used.contains(key) {
                pack.entries.insert(*key, value.clone());
            } else {
                summary.dropped += 1;
            }
        }
        summary.kept = pack.entries.len();
        summary.base = format!("{BASE_DIR}{}.pack", stamp());
        self.store.put(&summary.base, &pack.encode())?;

        // Delete what the new base supersedes. Failures are harmless: stale
        // deltas are skipped by readers (and re-covered by the next
        // compaction) and old bases are never read.
        for old in self.snapshot.base.iter().chain(covered.iter()) {
            match self.store.delete(old) {
                Ok(()) => summary.deleted += 1,
                Err(e) => log::warn!("failed to delete superseded cache object {old}: {e}"),
            }
        }
        Ok(summary)
    }

    /// Print cache statistics summary to stdout.
    pub fn print_stats(&self) {
        let (hits, misses, stores) = self.snapshot_stats();
        let total = hits + misses;
        let hit_pct = if total > 0 {
            hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let miss_pct = 100.0 - hit_pct;

        println!("========================== Cache statistics ===========================");
        println!("Mode:            {}", self.mode);
        println!("Store:           {}", self.store.describe());
        println!(
            "Base:            {}",
            self.snapshot.base.as_deref().unwrap_or("(none)")
        );
        println!("Deltas applied:  {}", self.snapshot.deltas.len());
        println!("Loaded entries:  {}", self.loaded.len());
        println!("Hits:            {hits} ({hit_pct:.1}%)");
        println!("Misses:          {misses} ({miss_pct:.1}%)");
        println!("New entries:     {stores}");
        println!("========================================================================");
    }

    /// Get a snapshot of current stats as (hits, misses, stores).
    pub fn snapshot_stats(&self) -> (usize, usize, usize) {
        (
            self.stats.hits.load(Relaxed),
            self.stats.misses.load(Relaxed),
            self.stats.stores.load(Relaxed),
        )
    }

    /// Number of entries loaded from the store.
    pub fn loaded_entries(&self) -> usize {
        self.loaded.len()
    }
}

/// Write a new base containing `entries` (hex SHA-256 key and response), for
/// seeding a store from another source such as the old directory-of-JSON
/// cache format. The base covers no deltas, so any existing deltas still
/// apply on top of it. Returns the new base's name.
pub fn write_seed_base(
    store: &dyn ObjectStore,
    entries: impl IntoIterator<Item = (String, serde_json::Value)>,
) -> io::Result<String> {
    let mut pack = Pack::default();
    for (hex_key, response) in entries {
        let key = (0..32)
            .map(|i| {
                hex_key
                    .get(2 * i..2 * i + 2)
                    .and_then(|b| u8::from_str_radix(b, 16).ok())
            })
            .collect::<Option<Vec<u8>>>()
            .and_then(|k| Key::try_from(k).ok())
            .ok_or_else(|| io::Error::other(format!("bad cache key {hex_key:?}")))?;
        pack.entries.insert(key, response.to_string());
    }
    let name = format!("{BASE_DIR}{}.pack", stamp());
    store.put(&name, &pack.encode())?;
    Ok(name)
}

/// A fresh object name stem that sorts by creation time: a zero-padded
/// millisecond timestamp plus a random suffix to keep concurrent writers
/// distinct.
fn stamp() -> String {
    use std::hash::{BuildHasher, Hasher};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut hasher = std::hash::RandomState::new().build_hasher();
    hasher.write_u128(now.as_nanos());
    hasher.write_u32(std::process::id());
    format!("{:016}-{:016x}", now.as_millis(), hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;
    use serde_json::json;

    fn open(store: &Arc<MemoryStore>, mode: CacheMode) -> Cache {
        Cache::open(store.clone(), mode).unwrap()
    }

    #[test]
    fn test_roundtrip() {
        let store = Arc::new(MemoryStore::default());
        let cache = open(&store, CacheMode::ReadWrite);

        cache
            .store("cvc5", "(set-logic ALL)\n(check-sat)", &json!("unsat"))
            .unwrap();

        let result = cache
            .lookup("cvc5", "(set-logic ALL)\n(check-sat)")
            .unwrap();
        assert_eq!(result, Some(json!("unsat")));

        // A different path is a different key.
        let result = cache
            .lookup("cvc5", "(set-logic QF_BV)\n(check-sat)")
            .unwrap();
        assert_eq!(result, None);

        // A different solver is a different key.
        let result = cache.lookup("z3", "(set-logic ALL)\n(check-sat)").unwrap();
        assert_eq!(result, None);

        // Flushed entries are visible to a later run.
        assert!(cache.flush(None).unwrap().is_some());
        let cache = open(&store, CacheMode::ReadOnlyEnforcing);
        let result = cache
            .lookup("cvc5", "(set-logic ALL)\n(check-sat)")
            .unwrap();
        assert_eq!(result, Some(json!("unsat")));
    }

    #[test]
    fn test_read_only_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let used_keys = dir.path().join("used-keys");
        let store = Arc::new(MemoryStore::default());
        let cache = open(&store, CacheMode::ReadOnly);
        cache.store("cvc5", "(check-sat)", &json!("sat")).unwrap();
        assert_eq!(
            cache.lookup("cvc5", "(check-sat)").unwrap(),
            Some(json!("sat"))
        );
        assert_eq!(cache.flush(Some(&used_keys)).unwrap(), None);
        assert!(store.list("").unwrap().is_empty());
        assert!(!used_keys.exists());
    }

    #[test]
    fn test_no_delta_without_new_entries() {
        let store = Arc::new(MemoryStore::default());
        let cache = open(&store, CacheMode::ReadWrite);
        assert_eq!(cache.flush(None).unwrap(), None);
        assert!(store.list("").unwrap().is_empty());
    }

    /// Runs that split one verification record their used keys in a shared
    /// file; a later compaction keeps exactly those entries, folds the
    /// deltas into a new base, and deletes the superseded objects.
    #[test]
    fn test_compaction() {
        let dir = tempfile::tempdir().unwrap();
        let used_keys = dir.path().join("used-keys");
        let store = Arc::new(MemoryStore::default());

        // Populate a, b, c in separate deltas.
        for q in ["a", "b", "c"] {
            let cache = open(&store, CacheMode::ReadWrite);
            cache.store("cvc5", q, &json!("sat")).unwrap();
            cache.flush(None).unwrap();
        }

        // Two runs over different parts of the workload: one uses `a`, the
        // other uses `b` and computes a new `d`.
        let first = open(&store, CacheMode::ReadWrite);
        assert!(first.lookup("cvc5", "a").unwrap().is_some());
        assert!(first.flush(Some(&used_keys)).unwrap().is_none());
        let second = open(&store, CacheMode::ReadWrite);
        assert!(second.lookup("cvc5", "b").unwrap().is_some());
        second.store("cvc5", "d", &json!("unsat")).unwrap();
        assert!(second.flush(Some(&used_keys)).unwrap().is_some());

        // Below the threshold, nothing happens.
        let compactor = open(&store, CacheMode::ReadWrite);
        assert_eq!(compactor.uncovered_deltas(), 4);
        assert_eq!(
            compactor
                .compact_with_used_keys_file(&used_keys, 5)
                .unwrap(),
            None
        );

        let summary = compactor
            .compact_with_used_keys_file(&used_keys, 4)
            .unwrap()
            .unwrap();
        assert_eq!((summary.kept, summary.dropped, summary.deleted), (3, 1, 4));
        assert_eq!(store.list("").unwrap(), vec![summary.base]);

        let cache = open(&store, CacheMode::ReadOnly);
        assert_eq!(cache.uncovered_deltas(), 0);
        for q in ["a", "b", "d"] {
            assert!(cache.lookup("cvc5", q).unwrap().is_some());
        }
        assert!(cache.lookup("cvc5", "c").unwrap().is_none());
    }

    /// Compaction refuses to run without any used keys, rather than drop
    /// everything.
    #[test]
    fn test_compaction_requires_used_keys() {
        let dir = tempfile::tempdir().unwrap();
        let used_keys = dir.path().join("used-keys");
        let store = Arc::new(MemoryStore::default());
        let cache = open(&store, CacheMode::ReadWrite);
        cache.store("cvc5", "a", &json!("sat")).unwrap();
        cache.flush(None).unwrap();

        let compactor = open(&store, CacheMode::ReadWrite);
        assert!(
            compactor
                .compact_with_used_keys_file(&used_keys, 1)
                .is_err()
        );
        std::fs::write(&used_keys, b"").unwrap();
        assert!(
            compactor
                .compact_with_used_keys_file(&used_keys, 1)
                .is_err()
        );
    }

    /// A delta written concurrently with a compaction (and so not covered by
    /// the new base) is still applied by later readers.
    #[test]
    fn test_concurrent_delta_survives_compaction() {
        let store = Arc::new(MemoryStore::default());
        let cache = open(&store, CacheMode::ReadWrite);
        cache.store("cvc5", "a", &json!("sat")).unwrap();
        cache.flush(None).unwrap();

        let compactor = open(&store, CacheMode::ReadWrite);
        let writer = open(&store, CacheMode::ReadWrite);
        writer.store("cvc5", "w", &json!("sat")).unwrap();
        writer.flush(None).unwrap();
        let used = [Cache::compute_key("cvc5", "a")].into_iter().collect();
        compactor.compact(&used).unwrap();

        let cache = open(&store, CacheMode::ReadOnlyEnforcing);
        assert_eq!(cache.loaded_entries(), 2);
        assert_eq!(cache.uncovered_deltas(), 1);
    }

    /// Covered deltas that survive (e.g. a failed delete) are skipped.
    #[test]
    fn test_stale_covered_delta_skipped() {
        let store = Arc::new(MemoryStore::default());
        let cache = open(&store, CacheMode::ReadWrite);
        cache.store("cvc5", "a", &json!("sat")).unwrap();
        let delta = cache.flush(None).unwrap().unwrap();
        let bytes = store.get(&delta).unwrap().unwrap();

        let used = [Cache::compute_key("cvc5", "a")].into_iter().collect();
        open(&store, CacheMode::ReadWrite).compact(&used).unwrap();
        // Resurrect the covered delta.
        store.put(&delta, &bytes).unwrap();

        let cache = open(&store, CacheMode::ReadWrite);
        assert_eq!(cache.uncovered_deltas(), 0);
        assert_eq!(cache.snapshot.stale_covered, vec![delta]);
    }
}
