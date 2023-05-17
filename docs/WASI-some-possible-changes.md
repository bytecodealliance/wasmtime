# Possible changes

The following are a list of relatively straightforward changes
to WASI core that should be considered.

## Split file/networking/random/clock from args/environ/exit.

Currently everything is mixed together in one big "core" module. But we can
split them out to allow minimal configurations that don't support this style
of files and networking.

## Move higher-level and unused errno codes out of the core API.

The core API currently defines errno codes such as `EDOM` which are
not used for anything. POSIX requires them to be defined, however
that can be done in the higher-level libraries, rather than in the
WASI core API itself.

## Detecting EOF from read/recv explicitly.

POSIX's `read` returns 0 if and only if it reaches the end of a file or stream.

Say you have a read buffer of 1024 bytes, and are reading a file that happens
to be 7 bytes long. The first `read` call will return 7, but unless you happen
to know how big the file is supposed to be, you can't distinguish between
that being all there is, and `read` getting interrupted and returning less
data than you requested.

Many applications today do an extra `read` when they encounter the end of a
file, to ensure that they get a `read` that returns 0 bytes read, to confirm
that they've reached the end of the file. If `read` instead had a way to
indicate that it had reached the end, this extra call wouldn't be necessary.

And, `read` on a socket is almost equivalent to `recv` with no flags -- except for
one surprising special case: on a datagram socket, if there's a zero-length
datagram, `read` can't consume it, while `recv` can. This is because `read` can't
indicate that it successfully read 0 bytes, because it has overloaded the meaning
of 0 to indicate eof-of-file.

So, it would be tidier from multiple perspectives if `read` could indicate
that it had reached the end of a file or stream, independently of how many
bytes it has read.

## Merging read and recv

These are very similar, and differ only in subtle ways. It'd make the API
easier to understand if they were unified.

## Trap instead of returning EFAULT

POSIX system calls return EFAULT when given invalid pointers, however from an
application perspective, it'd be more natural for them to just segfault.

## More detailed capability error reporting

Replace `__WASI_ENOTCAPABLE` with error codes that indicate *which* capabilities
were required but not present.

## Split `__wasi_path_open` into `__wasi_path_open_file` and `__wasi_path_open_directory`?

We could also split `__WASI_RIGHT_PATH_OPEN` into file vs directory,
(obviating `__WASI_O_DIRECTORY`).

## Fix the y2556 bug

In some places, timestamps are measured in nanoseconds since the UNIX epoch,
so our calculations indicate a 64-bit counter will overflow on
Sunday, July 21, 2554, at 11:34:33 pm UTC.

These timestamps aren't used in that many places, so it wouldn't cost that
much to widen these timestamps. We can either just extend the current type to
128 bits (two i64's in wasm) or move to a `timespec`-like `tv_sec`/`tv_nsec`
pair.

## Remove `fd_allocate`?

Darwin doesn't implement `posix_fallocate` (similar to `fd_allocate`), despite it being
in POSIX since 2001. So we don't currently know any way to implement `fd_allocate`
on Darwin that's safe from race conditions. Should we remove it from the API?

## Redesign `fstflags_t`

The relationship between `*_SET_*TIM` and `*_SET_*TIM_NOW` is non-obvious.
We should look at this again.

## readdir

Truncating entries that don't fit into a buffer may be error-prone. Should
we redesign how directory reading works?

## symlinks

Symlinks are fairly UNIX-specific. Should we remove `__wasi_path_symlink`
and `__wasi_path_readlink`?

Also, symlink resolution doesn't benefit from libpreopen-style path
translation. Should we move symlink resolution into the libpreopen layer
and do it entirely in "userspace"?

## Remove the `path_len` argument from `__wasi_fd_prestat_dir_name`

The buffer should be sized to the length returned from `__wasi_fd_prestat_get`,
so it's not necessary to pass the length back into the runtime.

## Add a `__wasi_path_filestat_set_size` function?

Along with libc/libpreopen support, this would enable implementing the
POSIX `truncate` function.

## errno values returned by `path_open`

We should specify the errno value returned when `path_open` is told
to open a directory and `__WASI_LOOKUP_SYMLINK_FOLLOW` isn't set, and
the path refers to a symbolic link.
