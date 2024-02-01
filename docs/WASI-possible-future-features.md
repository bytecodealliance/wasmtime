# Possible Future Features

These are some features we're interested in, but don't have yet, and which will
require some amount of design work.

## File Locking

POSIX's answer is `fcntl` with `F_SETLK`/`F_GETLK`/etc., which provide advisory
record locking. Unfortunately, these locks are associated with processes, which
means that if two parts of a program independently open a file and try to lock
it, if they're in the same process, they automatically share the lock.

Other locking APIs exist on various platforms, but none is widely standardized.

POSIX `F_SETLK`-style locking is used by SQLite.

## File change monitoring

POSIX has no performant way to monitor many files or directories for changes.

Many popular operating systems have system-specific APIs to do this though, so
it'd be desirable to come up with a portable API to provide access to this
functionality.

## Scalable event-based I/O

POSIX's `select` and `poll` have the property that each time they're called,
the implementation has to scan through all the file descriptors to report if any
of them has I/O ready, which is inefficient when there are large numbers of
open files or sockets.

Many popular operating systems have system-specific APIs that provide
alternative ways to monitor large numbers of I/O streams though, so it'd be
desirable to come up with a portable API to provide access to this
functionality.

## Crash recovery

POSIX doesn't have clear guidance on what applications can expect their
data will look like if the system crashes or the storage device is otherwise
taken offline abruptly.

We have `fsync` and `fdatasync`, but even these have been a topic of
[much discussion].

[much discussion]: https://wiki.postgresql.org/wiki/Fsync_Errors

Also, currently WASI's docs don't make any guarantees about things like
`path_rename` being atomic.
