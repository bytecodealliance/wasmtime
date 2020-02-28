# binary-compatible-builds

A small (ish) action which is intended to be used and will configure builds of
Rust projects to be "more binary compatible". On Windows and macOS this
involves setting a few env vars, and on Linux this involves spinning up a CentOS
6 container which is running in the background.

All subsequent build commands need to be wrapped in `$CENTOS` to optionally run
on `$CENTOS` on Linux to ensure builds happen inside the container.
