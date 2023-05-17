# Additional background on Capabilities

## Unforgeable references

One of the key words that describes capabilities is *unforgeable*.

A pointer in C is forgeable, because untrusted code could cast an integer
to a pointer, thus *forging* access to whatever that pointer value points
to.

MVP WebAssembly doesn't have unforgeable references, but what we can do instead
is just use integer values which are indices into a table that's held outside
the reach of untrusted code. The indices themselves are forgeable, but
ultimately the table is the thing which holds the actual capabilities, and
its elements are unforgeable. There's no way to gain access to a new resource
by making up a new index.

When the reference-types proposal lands, references will be unforgeable, and
will likely subsume the current integer-based APIs, at the WASI API layer.

## Static vs dynamic capabilities

There are two levels of capabilities that we can describe: static and dynamic.

The static capabilities of a wasm module are its imports. These essentially
declare the set of "rights" the module itself will be able to request.
An important caveat though is that this doesn't consider capabilities which
may be passed into an instance at runtime.

The dynamic capabilities of a wasm module are a set of boolean values
associated with a file descriptor, indicating individual "rights". This
includes things like the right to read, or to write, using a given file
descriptor.

## Filesystem rules

It happens that integer indices representing capabilities is same thing that
POSIX does, except that POSIX calls these indices *file descriptors*.

One difference though is that POSIX normally allows processes to request
a file descriptor for any file in the entire filesystem hierarchy, which is
granted based on whatever security policies are in place. This doesn't
violate the capability model, but it doesn't take full advantage of it.

CloudABI, Fuchsia, and other capability-oriented systems prefer to take
advantage of the hierarchical nature of the filesystem and require untrusted
code to have a capability for a directory in order to access things inside
that directory.

This way, you can launch untrusted code, and at runtime give it access to
specific directories, without having to set permissions in the filesystem or
in per-application or per-user configuration settings.

See [this tutorial](WASI-tutorial.md) for an example of how this can look
in practice.

## Berkeley socket rules

Sockets aren't naturally hierarchical though, so we'll need to decide what
capabilities look like. This is an area that isn't yet implemented.

In CloudABI, users launch programs with the sockets they need already
created. That's potentially a starting point, which might be enough for
simple cases.

We also anticipate an eventual extension to that, where we create a capability
that represents a set of possible sockets that can be created. A set
might be described by ranges of permitted ports, ranges of permitted
addresses, or sets of permitted protocols. In this case the actual socket
wouldn't be created until the application actually requests it.

## Other info

CloudABI's intro to capability-based OS security provides additional background info:

https://github.com/NuxiNL/cloudabi#capability-based-security


The Fuchsia project has a blog post on the topic of capability-based OS security:

https://fuchsia.dev/fuchsia-src/concepts/filesystems/dotdot
