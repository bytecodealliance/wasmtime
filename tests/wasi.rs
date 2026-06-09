//! Run the tests in `wasi_testsuite` using Wasmtime's CLI binary and checking
//! the results with a [wasi-testsuite] spec.
//!
//! [wasi-testsuite]: https://github.com/WebAssembly/wasi-testsuite

use http_body_util::BodyExt;
use libtest_mimic::{Arguments, Trial};
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Stdio};
use tempfile::TempDir;
use wasmtime::error::Context;
use wasmtime::{Result, ToWasmtimeResult as _, format_err};
use wit_component::ComponentEncoder;

const KNOWN_FAILURES: &[&str] = &[
    // FIXME(#11524)
    "remove_directory_trailing_slashes",
    // FIXME(#12475)
    "sockets-udp-send",
    #[cfg(target_vendor = "apple")]
    "filesystem-advise",
    // FIXME(WebAssembly/wasi-testsuite#128)
    #[cfg(windows)]
    "fd_fdstat_set_rights",
    #[cfg(windows)]
    "filesystem-flags-and-type",
    #[cfg(windows)]
    "path_link",
    #[cfg(windows)]
    "dangling_fd",
    #[cfg(windows)]
    "dangling_symlink",
    #[cfg(windows)]
    "file_allocate",
    #[cfg(windows)]
    "file_pread_pwrite",
    #[cfg(windows)]
    "file_seek_tell",
    #[cfg(windows)]
    "file_truncation",
    #[cfg(windows)]
    "file_unbuffered_write",
    #[cfg(windows)]
    "interesting_paths",
    #[cfg(windows)]
    "isatty",
    #[cfg(windows)]
    "fd_readdir",
    #[cfg(windows)]
    "nofollow_errors",
    #[cfg(windows)]
    "overwrite_preopen",
    #[cfg(windows)]
    "path_exists",
    #[cfg(windows)]
    "path_filestat",
    #[cfg(windows)]
    "path_open_create_existing",
    #[cfg(windows)]
    "path_open_dirfd_not_dir",
    #[cfg(windows)]
    "path_open_missing",
    #[cfg(windows)]
    "path_open_read_write",
    #[cfg(windows)]
    "path_rename",
    #[cfg(windows)]
    "path_rename_dir_trailing_slashes",
    #[cfg(windows)]
    "path_symlink_trailing_slashes",
    #[cfg(windows)]
    "readlink",
    #[cfg(windows)]
    "remove_nonempty_directory",
    #[cfg(windows)]
    "renumber",
    #[cfg(windows)]
    "symlink_create",
    #[cfg(windows)]
    "stdio",
    #[cfg(windows)]
    "symlink_filestat",
    #[cfg(windows)]
    "truncation_rights",
    #[cfg(windows)]
    "symlink_loop",
    #[cfg(windows)]
    "unlink_file_trailing_slashes",
    #[cfg(windows)]
    "filesystem-unlink-errors",
    #[cfg(windows)]
    "filesystem-stat",
    #[cfg(windows)]
    "filesystem-set-size",
    #[cfg(windows)]
    "filesystem-rename",
    #[cfg(windows)]
    "filesystem-read-directory",
    #[cfg(windows)]
    "filesystem-open-errors",
    #[cfg(windows)]
    "filesystem-mkdir-rmdir",
    #[cfg(windows)]
    "filesystem-metadata-hash",
    #[cfg(windows)]
    "filesystem-hard-links",
    #[cfg(windows)]
    "filesystem-io",
];

fn main() -> Result<()> {
    env_logger::init();

    let mut trials = Vec::new();
    if !cfg!(miri) {
        find_tests("tests/wasi-testsuite".as_ref(), &mut trials).unwrap();
    }

    libtest_mimic::run(&Arguments::from_args(), trials).exit()
}

fn find_tests(path: &Path, trials: &mut Vec<Trial>) -> Result<()> {
    for entry in path.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            find_tests(&path, trials)?;
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("wasm") {
            continue;
        }

        // Test the core wasm itself.
        trials.push(Trial::test(
            format!("wasmtime-wasi - {}", path.display()),
            {
                let path = path.clone();
                move || run_test(&path, false).map_err(|e| format!("{e:?}").into())
            },
        ));
    }
    Ok(())
}

fn run_test(path: &Path, componentize: bool) -> Result<()> {
    let wasmtime = Path::new(env!("CARGO_BIN_EXE_wasmtime"));
    let test_name = path.file_stem().unwrap().to_str().unwrap();
    let target_dir = wasmtime.parent().unwrap().parent().unwrap();
    let parent_dir = path.parent().ok_or(format_err!("module has no parent?"))?;
    let spec = if let Ok(contents) = fs::read_to_string(&path.with_extension("json")) {
        serde_json::from_str(&contents)?
    } else {
        Spec::default()
    };

    let mut td = TempDir::new_in(&target_dir)?;
    td.disable_cleanup(true);
    let path = if componentize {
        let module = fs::read(path).expect("read wasm module");
        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .to_wasmtime_result()?
            .validate(true)
            .adapter(
                "wasi_snapshot_preview1",
                &fs::read(test_programs_artifacts::ADAPTER_COMMAND)?,
            )
            .to_wasmtime_result()?
            .encode()
            .to_wasmtime_result()?;
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let component_path = td.path().join(format!("{stem}.component.wasm"));
        fs::write(&component_path, component)?;
        component_path
    } else {
        path.to_path_buf()
    };

    let spec_debug = format!("{spec:#?}");
    let spec_world = spec.world.clone();
    let mut child = KillOnDrop(None);
    let mut cmd_debug = String::new();
    let proposals = spec.proposals.clone();
    let mut streams = HashMap::new();
    let mut http_addr = None;
    let mut killed = false;

    for operation in spec.operations() {
        log::info!("execute: {operation:?}");
        match operation {
            Operation::Run { args, dirs, env } => {
                assert!(child.0.is_none());
                let mut cmd = wasmtime_test_util::command(wasmtime);
                match spec_world.as_deref() {
                    Some("wasi:http/service") => {
                        cmd.arg("serve");
                        cmd.arg("--addr=127.0.0.1:0");
                    }
                    Some(world) => panic!("unknown world {world}"),
                    None => {
                        cmd.arg("run");
                    }
                }
                for dir in dirs {
                    cmd.arg("--dir");
                    let src = parent_dir.join(&dir);
                    let dst = td.path().join(&dir);
                    cp_r(&src, &dst)?;
                    cmd.arg(format!("{}::{dir}", dst.display()));
                }
                for (k, v) in env {
                    cmd.arg("--env");
                    cmd.arg(format!("{k}={v}"));
                }
                if path.iter().any(|p| p == "wasm32-wasip3") {
                    cmd.arg("-Sp3").arg("-Wcomponent-model-async");
                }
                for proposal in proposals.as_deref().unwrap_or(&[]) {
                    match proposal {
                        WasiProposal::Sockets => {
                            cmd.arg("-Sinherit-network");
                        }
                        WasiProposal::Http => {
                            cmd.arg("-Shttp,cli");
                        }
                    };
                }
                cmd.arg(&path);
                cmd.args(args);
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
                cmd.stdin(Stdio::piped());

                cmd_debug = format!("{cmd:?}");
                child.0 = Some(cmd.spawn()?);
            }
            Operation::Write { id, payload } => {
                let child = child.0.as_mut().unwrap();
                let stream = match id {
                    StreamId::Stdin => child.stdin.as_mut().unwrap(),
                    StreamId::Stdout | StreamId::Stderr => {
                        panic!("cannot write to stdout or stderr")
                    }
                };
                stream.write_all(payload.as_bytes())?;
            }
            Operation::Read { id, payload } => {
                let child = child.0.as_mut().unwrap();
                let stream = match id {
                    StreamId::Stdout => child.stdout.as_mut().unwrap() as &mut dyn Read,
                    StreamId::Stderr => child.stderr.as_mut().unwrap() as &mut dyn Read,
                    StreamId::Stdin => panic!("cannot read from stdin"),
                };
                let mut buf = vec![0; payload.len()];
                stream.read_exact(&mut buf)?;
                if payload != String::from_utf8_lossy(&buf) {
                    wasmtime::bail!(
                        "unexpected output from {id:?}: expected {payload:?}, got {:?}",
                        String::from_utf8_lossy(&buf)
                    );
                }
            }
            Operation::Connect {
                id,
                protocol_type: ProtocolType::Tcp,
            } => {
                let child = child.0.as_mut().unwrap();
                let mut buf = [0; 200];
                let n = child.stdout.as_mut().unwrap().read(&mut buf)?;
                let addr = String::from_utf8_lossy(&buf[..n]);
                let stream = TcpStream::connect(addr.trim())?;
                let prev = streams.insert(id, stream);
                assert!(prev.is_none());
            }
            Operation::Send { id, payload } => {
                let stream = streams.get_mut(&id).unwrap();
                stream.write_all(payload.as_bytes())?;
            }
            Operation::Recv { id, payload } => {
                let stream = streams.get_mut(&id).unwrap();
                let mut buf = vec![0; payload.len()];
                stream.read_exact(&mut buf)?;
                if payload != String::from_utf8_lossy(&buf) {
                    wasmtime::bail!(
                        "unexpected output from stream {id:?}: expected {payload:?}, got {:?}",
                        String::from_utf8_lossy(&buf)
                    );
                }
            }
            Operation::Request { req } => {
                let http_addr = match &http_addr {
                    None => {
                        let child = child.0.as_mut().unwrap();
                        let mut buf = [0; 200];
                        let mut i = 0;
                        loop {
                            let n = child.stderr.as_mut().unwrap().read(&mut buf[i..])?;
                            assert!(n > 0);
                            i += n;
                            if buf[i - 1] == b'\n' {
                                break;
                            }
                        }
                        let addr = String::from_utf8_lossy(&buf[..i]);
                        let addr = addr
                            .trim()
                            .strip_prefix("Serving HTTP on http://")
                            .unwrap()
                            .strip_suffix("/")
                            .unwrap()
                            .parse::<SocketAddr>()
                            .unwrap();
                        http_addr = Some(addr);
                        addr
                    }
                    Some(addr) => *addr,
                };

                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()?
                    .block_on(send_request(http_addr, req))?;
            }
            Operation::Kill {
                signal: Signal::Sigint,
            } => {
                let child = child.0.as_mut().unwrap();
                child.kill()?;
                killed = true;
            }
            Operation::Wait {
                exit_code,
                stderr,
                stdout,
            } => {
                let result = child.0.take().unwrap().wait_with_output()?;
                td.disable_cleanup(true);
                let ok = (Some(exit_code.unwrap_or(0)) == result.status.code() || killed)
                    && matches_or_missing(&stdout, &result.stdout)
                    && matches_or_missing(&stderr, &result.stderr);
                let mut should_fail = KNOWN_FAILURES.contains(&test_name);
                if path.iter().any(|p| p == "wasm32-wasip3")
                    && !cfg!(feature = "component-model-async")
                {
                    should_fail = true;
                }

                match (ok, should_fail) {
                    // If this test passed and is not a known failure, or if it failed and
                    // it's a known failure, then flag this test as "ok".
                    (true, false) | (false, true) => {}

                    // If this test failed and it's not known to fail, explain why.
                    (false, false) => {
                        td.disable_cleanup(false);
                        let mut msg = String::new();
                        writeln!(msg, "  command: {cmd_debug}")?;
                        writeln!(msg, "  spec: {spec_debug}")?;
                        writeln!(msg, "  result.status: {}", result.status)?;
                        if !result.stdout.is_empty() {
                            write!(
                                msg,
                                "  result.stdout:\n    {}",
                                String::from_utf8_lossy(&result.stdout).replace("\n", "\n    ")
                            )?;
                        }
                        if !result.stderr.is_empty() {
                            writeln!(
                                msg,
                                "  result.stderr:\n    {}",
                                String::from_utf8_lossy(&result.stderr).replace("\n", "\n    ")
                            )?;
                        }
                        wasmtime::bail!(
                            "{msg}\nFAILED! The result does not match the specification"
                        );
                    }

                    // If this test passed but it's flagged as should be failed, then fail
                    // this test for someone to update `KNOWN_FAILURES`.
                    (true, true) => {
                        wasmtime::bail!("test passed but it's listed in `KNOWN_FAILURES`")
                    }
                }
            }
        }
    }
    assert!(child.0.is_none());
    Ok(())
}

fn cp_r(path: &Path, dst: &Path) -> Result<()> {
    fs::create_dir(dst)?;
    for entry in path.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        let dst = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            cp_r(&path, &dst)?;
        } else {
            fs::copy(&path, &dst)?;
        }
    }
    Ok(())
}

struct KillOnDrop(Option<Child>);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
        }
    }
}

/// Sends `HttpRequest` to `addr`, asserting the response matches the expected
/// response specified within `req`.
async fn send_request(addr: SocketAddr, req: HttpRequest) -> Result<()> {
    let tcp = TcpStream::connect(addr).with_context(|| format!("failed to connect to {addr:?}"))?;
    tcp.set_nonblocking(true)?;
    let tcp = tokio::net::TcpStream::from_std(tcp)?;
    let tcp = wasmtime_wasi_http::io::TokioIo::new(tcp);
    let (mut send, conn) = hyper::client::conn::http1::handshake(tcp)
        .await
        .context("failed http handshake")?;
    tokio::task::spawn(conn);
    let response = send
        .send_request(
            http::Request::builder()
                .method(http::Method::from(req.method))
                .uri(req.path)
                .body(String::new())
                .unwrap(),
        )
        .await
        .context("error sending request")?;
    let (parts, body) = response.into_parts();

    let body = body.collect().await.context("failed to read body")?;
    assert!(body.trailers().is_none());
    let body = std::str::from_utf8(&body.to_bytes())?.to_string();

    assert_eq!(parts.status.as_u16(), req.response.status);
    for header in &req.response.headers {
        let value = parts
            .headers
            .get(header.0.as_str())
            .ok_or_else(|| format_err!("missing header {} in response", header.0))?;
        if value != header.1.as_str() {
            wasmtime::bail!(
                "unexpected value for header {}: expected {:?}, got {:?}",
                header.0,
                header.1,
                value.to_str()?
            );
        }
    }
    if body != req.response.body {
        wasmtime::bail!(
            "unexpected body: expected {:?}, got {:?}",
            req.response.body,
            body
        );
    }

    Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    proposals: Option<Vec<WasiProposal>>,
    world: Option<String>,
    operations: Option<Vec<Operation>>,

    args: Option<Vec<String>>,
    dirs: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    exit_code: Option<i32>,
    stdout: Option<String>,
}

impl Spec {
    fn operations(mut self) -> Vec<Operation> {
        if let Some(ops) = self.operations.take() {
            assert!(self.args.is_none());
            assert!(self.dirs.is_none());
            assert!(self.env.is_none());
            assert!(self.exit_code.is_none());
            assert!(self.stdout.is_none());
            return ops;
        }

        vec![
            Operation::Run {
                args: self.args.take().unwrap_or_default(),
                dirs: self.dirs.take().unwrap_or_default(),
                env: self.env.take().unwrap_or_default(),
            },
            Operation::Wait {
                exit_code: self.exit_code,
                stderr: None,
                stdout: self.stdout.take(),
            },
        ]
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Run {
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        dirs: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    Write {
        id: StreamId,
        payload: String,
    },
    Read {
        id: StreamId,
        payload: String,
    },
    Connect {
        id: String,
        protocol_type: ProtocolType,
    },
    Send {
        id: String,
        payload: String,
    },
    Recv {
        id: String,
        payload: String,
    },
    Request {
        #[serde(flatten)]
        req: HttpRequest,
    },
    Kill {
        signal: Signal,
    },
    Wait {
        exit_code: Option<i32>,
        stderr: Option<String>,
        stdout: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StreamId {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProtocolType {
    Tcp,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum HttpMethod {
    Get,
    Post,
}

impl From<HttpMethod> for http::Method {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => http::Method::GET,
            HttpMethod::Post => http::Method::POST,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum Signal {
    Sigint,
}
#[derive(Debug, Deserialize)]
struct HttpRequest {
    method: HttpMethod,
    path: String,
    response: HttpResponse,
}

#[derive(Debug, Deserialize)]
struct HttpResponse {
    status: u16,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WasiProposal {
    Sockets,
    Http,
}

fn matches_or_missing(a: &Option<String>, b: &[u8]) -> bool {
    a.as_ref()
        .map(|s| s == &String::from_utf8_lossy(b))
        .unwrap_or(true)
}
