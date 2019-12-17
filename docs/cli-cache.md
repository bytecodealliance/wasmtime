# Cache Configuration of `wasmtime`

The configuration file uses the [toml] format.
You can create a configuration file at the default location with:
```
$ wasmtime config new
```
It will print the location regardless of the success.
Please refer to the  `--help` message for using a custom location.

All settings, except `enabled`, are **optional**.
If the setting is not specified, the **default** value is used.
***Thus, if you don't know what values to use, don't specify them.***
The default values might be tuned in the future.

Wasmtime assumes all the options are in the `cache` section.

Example config:
```toml
[cache]
enabled = true
directory = "/nfs-share/wasmtime-cache/"
cleanup-interval = "30m"
files-total-size-soft-limit = "1Gi"
```

Please refer to the [cache system] section to learn how it works.

If you think some default value should be tuned, some new settings
should be introduced or some behavior should be changed, you are
welcome to discuss it and contribute to [the Wasmtime repository].

[the Wasmtime repository]: https://github.com/bytecodealliance/wasmtime

Setting `enabled`
-----------------
- **type**: boolean
- **format**: `true | false`
- **default**: `true`

Specifies whether the cache system is used or not.

This field is *mandatory*.
The default value is used when configuration file is not specified
and none exists at the default location.

[`enabled`]: #setting-enabled

Setting `directory`
-----------------
- **type**: string (path)
- **default**: look up `cache_dir` in [directories] crate

Specifies where the cache directory is. Must be an absolute path.

[`directory`]: #setting-directory

Setting `worker-event-queue-size`
-----------------
- **type**: string (SI prefix)
- **format**: `"{integer}(K | M | G | T | P)?"`
- **default**: `"16"`

Size of [cache worker] event queue.
If the queue is full, incoming cache usage events will be dropped.

[`worker-event-queue-size`]: #setting-worker-event-queue-size

Setting `baseline-compression-level`
------------------
- **type**: integer
- **default**: `3`, the default zstd compression level

Compression level used when a new cache file is being written by the [cache system].
Wasmtime uses [zstd] compression.

[`baseline-compression-level`]: #setting-baseline-compression-level

Setting `optimized-compression-level`
------------------
- **type**: integer
- **default**: `20`

Compression level used when the [cache worker] decides to recompress a cache file.
Wasmtime uses [zstd] compression.

[`optimized-compression-level`]: #setting-optimized-compression-level

Setting `optimized-compression-usage-counter-threshold`
------------------
- **type**: string (SI prefix)
- **format**: `"{integer}(K | M | G | T | P)?"`
- **default**: `"256"`

One of the conditions for the [cache worker] to recompress a cache file
is to have usage count of the file exceeding this threshold.

[`optimized-compression-usage-counter-threshold`]: #setting-optimized-compression-usage-counter-threshold

Setting `cleanup-interval`
------------------
- **type**: string (duration)
- **format**: `"{integer}(s | m | h | d)"`
- **default**: `"1h"`

When the [cache worker] is notified about a cache file being updated by the [cache system]
and this interval has already passed since last cleaning up,
the worker will attempt a new cleanup.

Please also refer to [`allowed-clock-drift-for-files-from-future`].

[`cleanup-interval`]: #setting-cleanup-interval

Setting `optimizing-compression-task-timeout`
------------------
- **type**: string (duration)
- **format**: `"{integer}(s | m | h | d)"`
- **default**: `"30m"`

When the [cache worker] decides to recompress a cache file, it makes sure that
no other worker has started the task for this file within the last
[`optimizing-compression-task-timeout`] interval.
If some worker has started working on it, other workers are skipping this task.

Please also refer to the [`allowed-clock-drift-for-files-from-future`] section.

[`optimizing-compression-task-timeout`]: #setting-optimizing-compression-task-timeout

Setting `allowed-clock-drift-for-files-from-future`
------------------
- **type**: string (duration)
- **format**: `"{integer}(s | m | h | d)"`
- **default**: `"1d"`

### Locks
When the [cache worker] attempts acquiring a lock for some task,
it checks if some other worker has already acquired such a lock.
To be fault tolerant and eventually execute every task,
the locks expire after some interval.
However, because of clock drifts and different timezones,
it would happen that some lock was created in the future.
This setting defines a tolerance limit for these locks.
If the time has been changed in the system (i.e. two years backwards),
the [cache system] should still work properly.
Thus, these locks will be treated as expired
(assuming the tolerance is not too big).

### Cache files
Similarly to the locks, the cache files or their metadata might
have modification time in distant future.
The cache system tries to keep these files as long as possible.
If the limits are not reached, the cache files will not be deleted.
Otherwise, they will be treated as the oldest files, so they might survive.
If the user actually uses the cache file, the modification time will be updated.

[`allowed-clock-drift-for-files-from-future`]: #setting-allowed-clock-drift-for-files-from-future

Setting `file-count-soft-limit`
------------------
- **type**: string (SI prefix)
- **format**: `"{integer}(K | M | G | T | P)?"`
- **default**: `"65536"`

Soft limit for the file count in the cache directory.

This doesn't include files with metadata.
To learn more, please refer to the [cache system] section.

[`file-count-soft-limit`]: #setting-file-count-soft-limit

Setting `files-total-size-soft-limit`
------------------
- **type**: string (disk space)
- **format**: `"{integer}(K | Ki | M | Mi | G | Gi | T | Ti | P | Pi)?"`
- **default**: `"512Mi"`

Soft limit for the total size* of files in the cache directory.

This doesn't include files with metadata.
To learn more, please refer to the [cache system] section.

*this is the file size, not the space physically occupied on the disk.

[`files-total-size-soft-limit`]: #setting-files-total-size-soft-limit

Setting `file-count-limit-percent-if-deleting`
------------------
- **type**: string (percent)
- **format**: `"{integer}%"`
- **default**: `"70%"`

If [`file-count-soft-limit`] is exceeded and the [cache worker] performs the cleanup task,
then the worker will delete some cache files, so after the task,
the file count should not exceed
[`file-count-soft-limit`] * [`file-count-limit-percent-if-deleting`].

This doesn't include files with metadata.
To learn more, please refer to the [cache system] section.

[`file-count-limit-percent-if-deleting`]: #setting-file-count-limit-percent-if-deleting

Setting `files-total-size-limit-percent-if-deleting`
------------------
- **type**: string (percent)
- **format**: `"{integer}%"`
- **default**: `"70%"`

If [`files-total-size-soft-limit`] is exceeded and [cache worker] performs the cleanup task,
then the worker will delete some cache files, so after the task,
the files total size should not exceed
[`files-total-size-soft-limit`] * [`files-total-size-limit-percent-if-deleting`].

This doesn't include files with metadata.
To learn more, please refer to the [cache system] section.

[`files-total-size-limit-percent-if-deleting`]: #setting-files-total-size-limit-percent-if-deleting

[toml]: https://github.com/toml-lang/toml
[directories]: https://crates.io/crates/directories
[cache system]: #how-does-the-cache-work
[cache worker]: #how-does-the-cache-work
[zstd]: https://facebook.github.io/zstd/
[Least Recently Used (LRU)]: https://en.wikipedia.org/wiki/Cache_replacement_policies#Least_recently_used_(LRU)

How does the cache work?
========================

**This is an implementation detail and might change in the future.**
Information provided here is meant to help understanding the big picture
and configuring the cache.

There are two main components - the *cache system* and the *cache worker*.

Cache system
------------

Handles GET and UPDATE cache requests.
- **GET request** - simply loads the cache from disk if it is there.
- **UPDATE request** - compresses received data with [zstd] and [`baseline-compression-level`], then writes the data to the disk.

In case of successful handling of a request, it notifies the *cache worker* about this
event using the queue.
The queue has a limited size of [`worker-event-queue-size`]. If it is full, it will drop
new events until the *cache worker* pops some event from the queue.

Cache worker
------------

The cache worker runs in a single thread with lower priority and pops events from the queue
in a loop handling them one by one.

### On GET request
1. Read the statistics file for the cache file,
   increase the usage counter and write it back to the disk.
2. Attempt recompressing the cache file if all of the following conditions are met:
   - usage counter exceeds [`optimized-compression-usage-counter-threshold`],
   - the file is compressed with compression level lower than [`optimized-compression-level`],
   - no other worker has started working on this particular task within the last
     [`optimizing-compression-task-timeout`] interval.

   When recompressing, [`optimized-compression-level`] is used as a compression level.

### On UPDATE request
1. Write a fresh statistics file for the cache file.
2. Clean up the cache if no worker has attempted to do this within the last [`cleanup-interval`].
   During this task:
   - all unrecognized files and expired task locks in cache directory will be deleted
   - if [`file-count-soft-limit`] or [`files-total-size-soft-limit`] is exceeded,
     then recognized files will be deleted according to
     [`file-count-limit-percent-if-deleting`] and [`files-total-size-limit-percent-if-deleting`].
     Wasmtime uses [Least Recently Used (LRU)] cache replacement policy and requires that
     the filesystem maintains proper mtime (modification time) of the files.
     Files with future mtimes are treated specially - more details
     in [`allowed-clock-drift-for-files-from-future`].

### Metadata files
- every cached WebAssembly module has its own statistics file
- every lock is a file
