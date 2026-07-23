//! The caching SMT context: an [`easy_smt::Context`]-compatible wrapper.

use std::{
    ffi::OsString,
    io,
    sync::{Arc, Mutex},
};

use easy_smt::{Response, SExpr, SExprData};

use crate::{
    cache::{Cache, CacheMode},
    convert,
};

/// Builder for a caching [`Context`], mirroring [`easy_smt::ContextBuilder`].
///
/// Unlike `easy_smt`, configuring a solver here does *not* spawn it at
/// [`build`](Self::build) time: the solver is launched lazily, on the first
/// query that misses the cache.
#[derive(Default)]
pub struct ContextBuilder {
    solver: Option<OsString>,
    solver_args: Vec<OsString>,
    replay_file: Option<Box<dyn io::Write + Send>>,
    cache: Option<Arc<Cache>>,
}

impl ContextBuilder {
    /// Construct a new builder with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the solver that will be used on a cache miss.
    pub fn solver<P>(&mut self, program: P) -> &mut Self
    where
        P: Into<OsString>,
    {
        self.solver = Some(program.into());
        self
    }

    /// Configure the arguments that will be passed to the solver.
    pub fn solver_args<A>(&mut self, args: A) -> &mut Self
    where
        A: IntoIterator,
        A::Item: Into<OsString>,
    {
        self.solver_args = args.into_iter().map(|a| a.into()).collect();
        self
    }

    /// An optional file (or anything else that is `std::io::Write`-able) where
    /// all commands sent to a live solver are tee'd to. Nothing is written if
    /// every query is served from the cache.
    pub fn replay_file<W>(&mut self, replay_file: Option<W>) -> &mut Self
    where
        W: 'static + io::Write + Send,
    {
        self.replay_file = replay_file.map(|w| Box::new(w) as _);
        self
    }

    /// Configure the query cache. Without a cache every query is a miss, so
    /// the context behaves like a (lazily spawned) plain solver context.
    pub fn cache(&mut self, cache: Arc<Cache>) -> &mut Self {
        self.cache = Some(cache);
        self
    }

    /// Finish configuring the context and build it. No solver is spawned.
    pub fn build(&mut self) -> io::Result<Context> {
        Ok(Context {
            inner: easy_smt::ContextBuilder::new().build()?,
            solver: self.solver.take(),
            solver_args: std::mem::take(&mut self.solver_args),
            replay_file: self.replay_file.take().map(|w| Arc::new(Mutex::new(w))),
            cache: self.cache.take(),
            frames: vec![Vec::new()],
            live: None,
            pending: None,
        })
    }
}

/// A shared handle to the replay file, so that each lazily spawned solver
/// context (there may be several over this context's lifetime) can tee into
/// the same underlying writer.
struct SharedWriter(Arc<Mutex<Box<dyn io::Write + Send>>>);

impl io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// One command on the recorded path: the expression (in the wrapper context's
/// arena, for replaying into a spawned solver) and its display text (for
/// deriving cache keys).
struct PathCmd {
    expr: SExpr,
    text: String,
}

/// An SMT-LIB2 context with transparent query caching.
///
/// Expression-building methods are those of [`easy_smt::Context`], available
/// through `Deref`. Command methods (`assert`, `push`, `check`, `raw_send`,
/// ...) are intercepted: the tree-path of commands up to the current point is
/// recorded, queries are answered from the cache when possible, and a real
/// solver is spawned — and the recorded path played into it — only on a cache
/// miss.
pub struct Context {
    /// Solver-less easy-smt context, used as the s-expression arena.
    inner: easy_smt::Context,
    /// Solver program to launch on a cache miss.
    solver: Option<OsString>,
    /// Arguments for the solver program.
    solver_args: Vec<OsString>,
    /// Shared replay tee for all solver sessions of this context.
    replay_file: Option<Arc<Mutex<Box<dyn io::Write + Send>>>>,
    /// The query cache. `None` disables caching (every query misses).
    cache: Option<Arc<Cache>>,
    /// The tree-path of commands: a stack of frames delimited by `push`/`pop`.
    /// `frames[0]` is the base frame; each later frame corresponds to an open
    /// `push`.
    frames: Vec<Vec<PathCmd>>,
    /// Live solver session, if one has been spawned by a cache miss. Its state
    /// always mirrors a replay of `frames`; it is dropped whenever that stops
    /// being maintainable by appending (on `pop`, or when a query is answered
    /// from the cache instead of being forwarded).
    live: Option<easy_smt::Context>,
    /// Response queued by `raw_send` for the next `raw_recv`.
    pending: Option<SExpr>,
}

impl Context {
    // --- raw command interface, mirroring `easy_smt::Context`.

    /// Directly send a command to the (logical) solver.
    ///
    /// The command is processed immediately — recorded, answered from cache,
    /// or forwarded to a live solver — and its response is queued for the next
    /// [`raw_recv`](Self::raw_recv), which must be called before the next
    /// `raw_send`.
    pub fn raw_send(&mut self, cmd: SExpr) -> io::Result<()> {
        if self.pending.is_some() {
            return Err(io::Error::other(
                "raw_send called with an unconsumed pending response",
            ));
        }
        let resp = self.dispatch(cmd)?;
        self.pending = Some(resp);
        Ok(())
    }

    /// Receive the response to the last [`raw_send`](Self::raw_send).
    pub fn raw_recv(&mut self) -> io::Result<SExpr> {
        self.pending
            .take()
            .ok_or_else(|| io::Error::other("raw_recv called with no pending response"))
    }

    // --- high-level command interface, mirroring `easy_smt::Context`.

    pub fn set_option<K>(&mut self, name: K, value: SExpr) -> io::Result<()>
    where
        K: Into<String> + AsRef<str>,
    {
        let cmd = self.inner.list(vec![
            self.inner.atoms().set_option,
            self.inner.atom(name),
            value,
        ]);
        self.ack(cmd)
    }

    pub fn set_logic<L: Into<String> + AsRef<str>>(&mut self, logic: L) -> io::Result<()> {
        let cmd = self
            .inner
            .list(vec![self.inner.atoms().set_logic, self.inner.atom(logic)]);
        self.ack(cmd)
    }

    pub fn declare_sort<S: Into<String> + AsRef<str>>(
        &mut self,
        symbol: S,
        arity: i32,
    ) -> io::Result<SExpr> {
        let symbol = self.inner.atom(symbol);
        let arity = self.inner.numeral(arity);
        let cmd = self
            .inner
            .list(vec![self.inner.atoms().declare_sort, symbol, arity]);
        self.ack(cmd)?;
        Ok(symbol)
    }

    /// Declare a new constant with the provided sort.
    pub fn declare_const<S: Into<String> + AsRef<str>>(
        &mut self,
        name: S,
        sort: SExpr,
    ) -> io::Result<SExpr> {
        let name = self.inner.atom(name);
        let cmd = self
            .inner
            .list(vec![self.inner.atoms().declare_const, name, sort]);
        self.ack(cmd)?;
        Ok(name)
    }

    /// Declares a new, uninterpreted function symbol.
    pub fn declare_fun<S: Into<String> + AsRef<str>>(
        &mut self,
        name: S,
        args: Vec<SExpr>,
        out: SExpr,
    ) -> io::Result<SExpr> {
        let name = self.inner.atom(name);
        let cmd = self.inner.list(vec![
            self.inner.atoms().declare_fun,
            name,
            self.inner.list(args),
            out,
        ]);
        self.ack(cmd)?;
        Ok(name)
    }

    /// Defines a new function with a body.
    pub fn define_fun<S: Into<String> + AsRef<str>>(
        &mut self,
        name: S,
        args: Vec<(S, SExpr)>,
        out: SExpr,
        body: SExpr,
    ) -> io::Result<SExpr> {
        let name = self.inner.atom(name);
        let args = args
            .into_iter()
            .map(|(n, s)| self.inner.list(vec![self.inner.atom(n), s]))
            .collect();
        let cmd = self.inner.list(vec![
            self.inner.atoms().define_fun,
            name,
            self.inner.list(args),
            out,
            body,
        ]);
        self.ack(cmd)?;
        Ok(name)
    }

    /// Define a constant with a value. A convenience wrapper over
    /// [`Self::define_fun`] since constants are nullary functions.
    pub fn define_const<S: Into<String> + AsRef<str>>(
        &mut self,
        name: S,
        out: SExpr,
        body: SExpr,
    ) -> io::Result<SExpr> {
        self.define_fun(name, vec![], out, body)
    }

    pub fn assert(&mut self, expr: SExpr) -> io::Result<()> {
        let cmd = self.inner.list(vec![self.inner.atoms().assert, expr]);
        self.ack(cmd)
    }

    /// Push a new context frame. Same as SMT-LIB's `push` command.
    pub fn push(&mut self) -> io::Result<()> {
        let cmd = self.inner.list(vec![self.inner.atoms().push]);
        self.ack(cmd)
    }

    pub fn push_many(&mut self, n: usize) -> io::Result<()> {
        let cmd = self
            .inner
            .list(vec![self.inner.atoms().push, self.inner.numeral(n)]);
        self.ack(cmd)
    }

    /// Pop a context frame. Same as SMT-LIB's `pop` command.
    pub fn pop(&mut self) -> io::Result<()> {
        let cmd = self.inner.list(vec![self.inner.atoms().pop]);
        self.ack(cmd)
    }

    pub fn pop_many(&mut self, n: usize) -> io::Result<()> {
        let cmd = self
            .inner
            .list(vec![self.inner.atoms().pop, self.inner.numeral(n)]);
        self.ack(cmd)
    }

    /// Assert `check-sat` for the current context.
    pub fn check(&mut self) -> io::Result<Response> {
        let cmd = self.inner.list(vec![self.inner.atoms().check_sat]);
        let resp = self.dispatch(cmd)?;
        self.response(resp)
    }

    /// Assert `check-sat-assuming` with the given list of assumptions.
    pub fn check_assuming(
        &mut self,
        props: impl IntoIterator<Item = SExpr>,
    ) -> io::Result<Response> {
        let args = self.inner.list(props.into_iter().collect());
        let cmd = self
            .inner
            .list(vec![self.inner.atoms().check_sat_assuming, args]);
        let resp = self.dispatch(cmd)?;
        self.response(resp)
    }

    /// Get a model out from the solver. Only meaningful after a `check-sat`
    /// query that returned `sat`.
    pub fn get_model(&mut self) -> io::Result<SExpr> {
        let cmd = self.inner.list(vec![self.inner.atoms().get_model]);
        self.dispatch(cmd)
    }

    /// Get bindings for values in the model. Only meaningful after a
    /// `check-sat` query that returned `sat`.
    pub fn get_value(&mut self, vals: Vec<SExpr>) -> io::Result<Vec<(SExpr, SExpr)>> {
        if vals.is_empty() {
            return Ok(vec![]);
        }
        let cmd = self
            .inner
            .list(vec![self.inner.atoms().get_value, self.inner.list(vals)]);
        let resp = self.dispatch(cmd)?;
        match self.inner.get(resp) {
            SExprData::List(pairs) => {
                let mut res = Vec::with_capacity(pairs.len());
                for expr in pairs {
                    match self.inner.get(*expr) {
                        SExprData::List(pair) if pair.len() == 2 => {
                            res.push((pair[0], pair[1]));
                        }
                        _ => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "failed to parse get-value response",
                            ));
                        }
                    }
                }
                Ok(res)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to parse get-value response",
            )),
        }
    }

    /// Returns the names of the formulas involved in a contradiction.
    pub fn get_unsat_core(&mut self) -> io::Result<SExpr> {
        let cmd = self.inner.list(vec![self.inner.atoms().get_unsat_core]);
        self.dispatch(cmd)
    }

    /// Instruct the (logical) solver to exit. If no live solver is running,
    /// this is a no-op.
    pub fn exit(&mut self) -> io::Result<()> {
        let cmd = self.inner.list(vec![self.inner.atoms().exit]);
        let _ = self.dispatch(cmd)?;
        Ok(())
    }

    // --- internals.

    /// Process one command: record it on the path and produce its response,
    /// consulting the cache and live solver as appropriate.
    fn dispatch(&mut self, cmd: SExpr) -> io::Result<SExpr> {
        enum Kind {
            Frame,
            Query,
            Exit,
            Ack,
        }

        let (kind, arg) = {
            // Classify by head atom. A bare atom command (rare) is treated
            // like a single-element list.
            let (head, arg) = match self.inner.get(cmd) {
                SExprData::Atom(a) => (Some(a), None),
                SExprData::List(items) => (
                    items.first().and_then(|e| self.inner.get_atom(*e)),
                    items.get(1).and_then(|e| self.inner.get_atom(*e)),
                ),
                SExprData::String(_) => (None, None),
            };
            let arg: Option<usize> = arg.and_then(|a| a.parse().ok());
            let kind = match head {
                Some("push") | Some("pop") => Kind::Frame,
                Some(
                    "check-sat" | "check-sat-using" | "check-sat-assuming" | "get-value"
                    | "get-model" | "get-unsat-core" | "get-info" | "get-proof",
                ) => Kind::Query,
                Some("exit") => Kind::Exit,
                _ => Kind::Ack,
            };
            (kind, arg)
        };

        let text = self.inner.display(cmd).to_string();
        match kind {
            Kind::Frame => {
                let n = arg.unwrap_or(1);
                if text.starts_with("(push") {
                    self.frame_push(n, cmd)
                } else {
                    self.frame_pop(n)
                }
            }
            Kind::Query => self.query(cmd, &text),
            Kind::Exit => self.exit_live(),
            Kind::Ack => self.ack_command(cmd, text),
        }
    }

    /// Handle a `push`: open `n` new frames.
    fn frame_push(&mut self, n: usize, cmd: SExpr) -> io::Result<SExpr> {
        for _ in 0..n {
            self.frames.push(Vec::new());
        }
        if self.live.is_some() {
            self.forward(cmd)?;
        }
        Ok(self.inner.atoms().success)
    }

    /// Handle a `pop`: discard the top `n` frames.
    ///
    /// Any live solver is dropped: the caching layer deliberately does not
    /// reuse internal solver state across a pop. A later miss respawns a
    /// solver and replays the (now shorter) path.
    fn frame_pop(&mut self, n: usize) -> io::Result<SExpr> {
        if self.frames.len() <= n {
            return Err(io::Error::other(format!(
                "pop {n} with only {} frame(s) open",
                self.frames.len() - 1
            )));
        }
        self.frames.truncate(self.frames.len() - n);
        self.drop_live("pop");
        Ok(self.inner.atoms().success)
    }

    /// Handle a query: serve from cache or from a (lazily spawned) solver.
    fn query(&mut self, cmd: SExpr, text: &str) -> io::Result<SExpr> {
        let script = self.script_with(text);
        let solver_name = self.solver_name();

        // Consult the cache first, even if a live solver is running from an
        // earlier miss: a hit avoids re-solving. Serving a hit desyncs any
        // live solver (it has not seen this query), so drop it.
        if let Some(cache) = &self.cache {
            if let Some(response) = cache.lookup(&solver_name, &script)? {
                self.drop_live("cache hit");
                self.record(cmd, text);
                return convert::from_json(&self.inner, &response);
            }
            if cache.mode() == CacheMode::ReadOnlyEnforcing {
                return Err(io::Error::other(format!(
                    "SMT cache miss in read-only-enforcing mode: no cached response \
                     for query {text}",
                )));
            }
        }

        // Miss: make sure a live solver exists with the path played into it,
        // then forward the query.
        if self.live.is_none() {
            self.spawn_and_replay()?;
        }
        let response = self.forward(cmd)?;
        if let Some(cache) = &self.cache {
            cache.store(
                &solver_name,
                &script,
                &convert::to_json(&self.inner, response),
            )?;
        }
        self.record(cmd, text);
        Ok(response)
    }

    /// Handle an acknowledged command (declare, assert, set-logic, ...):
    /// record it on the path, forwarding to the live solver if there is one.
    fn ack_command(&mut self, cmd: SExpr, text: String) -> io::Result<SExpr> {
        if self.live.is_some() {
            let response = self.forward(cmd)?;
            self.record(cmd, &text);
            return Ok(response);
        }
        self.record(cmd, &text);
        Ok(self.inner.atoms().success)
    }

    /// Handle `exit`: shut down any live solver. Never spawns one.
    fn exit_live(&mut self) -> io::Result<SExpr> {
        if let Some(mut live) = self.live.take() {
            // Attempt a clean shutdown; the subprocess is killed on drop
            // regardless.
            let _ = live.exit();
        }
        Ok(self.inner.atoms().success)
    }

    /// Send one command to the live solver and return its response, copied
    /// back into this context's arena.
    fn forward(&mut self, cmd: SExpr) -> io::Result<SExpr> {
        let live = self.live.as_ref().expect("forward requires a live solver");
        let sent = convert::copy(&self.inner, live, cmd);
        let live = self.live.as_mut().unwrap();
        live.raw_send(sent)?;
        let resp = live.raw_recv()?;
        Ok(convert::copy(live, &self.inner, resp))
    }

    /// Append a command to the innermost open frame.
    fn record(&mut self, expr: SExpr, text: &str) {
        self.frames
            .last_mut()
            .expect("base frame always exists")
            .push(PathCmd {
                expr,
                text: text.to_string(),
            });
    }

    /// The replay commands for the current path: the base frame's commands,
    /// then for each open frame a `(push)` followed by that frame's commands.
    fn replay_cmds(&self) -> Vec<SExpr> {
        let push = self.inner.list(vec![self.inner.atoms().push]);
        let mut cmds = Vec::new();
        for (i, frame) in self.frames.iter().enumerate() {
            if i > 0 {
                cmds.push(push);
            }
            cmds.extend(frame.iter().map(|c| c.expr));
        }
        cmds
    }

    /// The cache-key script: the display text of the replay commands followed
    /// by the query command.
    fn script_with(&self, query: &str) -> String {
        let mut script = String::new();
        for (i, frame) in self.frames.iter().enumerate() {
            if i > 0 {
                script.push_str("(push)\n");
            }
            for c in frame {
                script.push_str(&c.text);
                script.push('\n');
            }
        }
        script.push_str(query);
        script
    }

    /// The solver's name for cache-key purposes: the file name of the solver
    /// program (e.g. "cvc5"), so that keys do not depend on install paths or
    /// invocation arguments (such as per-query timeouts).
    fn solver_name(&self) -> String {
        self.solver
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Spawn a solver subprocess and play the recorded path into it.
    fn spawn_and_replay(&mut self) -> io::Result<()> {
        let program = self
            .solver
            .clone()
            .ok_or_else(|| io::Error::other("cache miss but no solver is configured"))?;

        let replay_cmds = self.replay_cmds();
        if let Some(replay) = &self.replay_file {
            // Sessions are separated in the replay tee so that a replay file
            // containing several solver sessions remains readable.
            let _ = writeln!(
                replay.lock().unwrap(),
                "; [cache] solver session start: replaying {} command(s)",
                replay_cmds.len()
            );
        }

        let mut builder = easy_smt::ContextBuilder::new();
        builder
            .solver(program)
            .solver_args(self.solver_args.clone());
        if let Some(replay) = &self.replay_file {
            builder.replay_file(Some(SharedWriter(replay.clone())));
        }
        // `build` spawns the subprocess and sets the standard options
        // (:print-success, :produce-models), so every subsequent command —
        // including replayed ones — elicits exactly one response.
        let mut live = builder.build()?;

        for cmd in replay_cmds {
            let sent = convert::copy(&self.inner, &live, cmd);
            live.raw_send(sent)?;
            let resp = live.raw_recv()?;
            // Replayed commands answer `success` (acks) or a query answer
            // (`sat`/`unsat`/...), both of which are discarded; an `(error
            // ...)` response means the replayed state is broken.
            if let SExprData::List(items) = live.get(resp)
                && items.first().and_then(|e| live.get_atom(*e)) == Some("error")
            {
                let display = live.display(resp).to_string();
                let cmd = self.inner.display(cmd);
                return Err(io::Error::other(format!(
                    "solver error while replaying cached path at {cmd}: {display}",
                )));
            }
        }

        self.live = Some(live);
        Ok(())
    }

    /// Drop the live solver, if any.
    fn drop_live(&mut self, why: &str) {
        if let Some(live) = self.live.take() {
            drop(live);
            if let Some(replay) = &self.replay_file {
                let _ = writeln!(replay.lock().unwrap(), "; [cache] solver dropped ({why})");
            }
            log::debug!("dropped live solver ({why})");
        }
    }

    /// Issue a command whose expected response is `success`.
    fn ack(&mut self, cmd: SExpr) -> io::Result<()> {
        let resp = self.dispatch(cmd)?;
        if resp == self.inner.atoms().success {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "unexpected solver response: {}",
                self.inner.display(resp)
            )))
        }
    }

    /// Interpret a response as a check-sat [`Response`].
    fn response(&self, resp: SExpr) -> io::Result<Response> {
        let atoms = self.inner.atoms();
        if resp == atoms.sat {
            Ok(Response::Sat)
        } else if resp == atoms.unsat {
            Ok(Response::Unsat)
        } else if resp == atoms.unknown {
            Ok(Response::Unknown)
        } else {
            Err(io::Error::other(format!(
                "unexpected result from solver: {}",
                self.inner.display(resp)
            )))
        }
    }
}

use std::io::Write as _;

/// All `&self` expression-building and inspection methods of
/// [`easy_smt::Context`] (`atom`, `list`, `numeral`, `display`, `get`,
/// `atoms`, the sort constructors, and the operator helpers) are available
/// directly on [`Context`] through deref.
///
/// The deref target is deliberately immutable: `easy_smt`'s *command* methods
/// take `&mut self` and are therefore unreachable through it. The caching
/// interceptions above are the only way to issue commands.
impl std::ops::Deref for Context {
    type Target = easy_smt::Context;

    fn deref(&self) -> &easy_smt::Context {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{Cache, CacheMode};
    use std::path::PathBuf;

    /// Build a context around a solver that cannot possibly be spawned, so
    /// tests prove that fully cached runs never launch a solver.
    fn unspawnable_context(cache: Option<Arc<Cache>>) -> Context {
        let mut builder = ContextBuilder::new();
        builder
            .solver("/nonexistent/definitely-not-a-solver")
            .solver_args(["--nope"]);
        if let Some(cache) = cache {
            builder.cache(cache);
        }
        builder.build().unwrap()
    }

    /// Drive a small verification-shaped session against a context:
    /// prelude, declaration, an assert, then a pushed frame with a check-sat
    /// and (on sat) a get-value, then pop and exit. Returns the check
    /// response and the get-value bindings.
    fn session(ctx: &mut Context) -> io::Result<(Response, Vec<(String, String)>)> {
        ctx.set_logic("ALL")?;
        let bv8 = ctx.bit_vec_sort(ctx.numeral(8));
        let x = ctx.declare_const("x", bv8)?;
        let zero = ctx.binary(8, 0);
        ctx.assert(ctx.eq(x, zero))?;
        ctx.push()?;
        let not_eq = ctx.not(ctx.eq(x, zero));
        ctx.assert(not_eq)?;
        let resp = ctx.check()?;
        let values = if resp == Response::Sat {
            ctx.get_value(vec![x])?
                .into_iter()
                .map(|(k, v)| (ctx.display(k).to_string(), ctx.display(v).to_string()))
                .collect()
        } else {
            vec![]
        };
        ctx.pop()?;
        ctx.exit()?;
        Ok((resp, values))
    }

    /// A path to a fake solver: a shell script that speaks just enough of the
    /// SMT-LIB2 protocol (with :print-success) to answer a session.
    #[cfg(unix)]
    fn fake_solver(dir: &std::path::Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake-solver.sh");
        std::fs::write(
            &path,
            r#"#!/bin/sh
while IFS= read -r line; do
    case "$line" in
        "(check-sat)") echo "sat" ;;
        "(get-value"*) echo "((x #b00000000))" ;;
        "(exit)") echo "success"; exit 0 ;;
        *) echo "success" ;;
    esac
done
"#,
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// End-to-end: a first session against a (fake) live solver populates the
    /// cache; a second session with an unspawnable solver is served entirely
    /// from the cache, including the get-value model.
    #[cfg(unix)]
    #[test]
    fn test_populate_then_replay_from_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let cache = Arc::new(Cache::open(
            None,
            Some(cache_dir.clone()),
            CacheMode::ReadWrite,
        ));

        // First run: misses; spawns the fake solver.
        let mut ctx = ContextBuilder::new()
            .solver(fake_solver(dir.path()))
            .cache(cache.clone())
            .build()
            .unwrap();
        let (resp, values) = session(&mut ctx).unwrap();
        assert_eq!(resp, Response::Sat);
        assert_eq!(values, vec![("x".to_string(), "#b00000000".to_string())]);
        let (hits, misses, stores, _) = cache.snapshot_stats();
        assert_eq!((hits, misses, stores), (0, 2, 2));

        // Second run: everything is served from the cache; the "solver" is
        // unspawnable, but must be named like the fake solver so the cache
        // keys match. No solver is launched.
        let unspawnable = PathBuf::from("/nonexistent/dir/fake-solver.sh");
        let cache = Arc::new(Cache::open(
            Some(cache_dir),
            None,
            CacheMode::ReadOnlyEnforcing,
        ));
        let mut ctx = ContextBuilder::new()
            .solver(unspawnable)
            .cache(cache.clone())
            .build()
            .unwrap();
        let (resp, values) = session(&mut ctx).unwrap();
        assert_eq!(resp, Response::Sat);
        assert_eq!(values, vec![("x".to_string(), "#b00000000".to_string())]);
        let (hits, misses, _, _) = cache.snapshot_stats();
        assert_eq!((hits, misses), (2, 0));
    }

    /// The same query issued under different paths (different asserted state)
    /// gets different cache entries.
    #[cfg(unix)]
    #[test]
    fn test_path_distinguishes_queries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Arc::new(Cache::open(
            None,
            Some(dir.path().join("cache")),
            CacheMode::ReadWrite,
        ));

        let mut ctx = ContextBuilder::new()
            .solver(fake_solver(dir.path()))
            .cache(cache.clone())
            .build()
            .unwrap();
        ctx.set_logic("ALL").unwrap();
        ctx.push().unwrap();
        ctx.assert(ctx.true_()).unwrap();
        ctx.check().unwrap();
        ctx.pop().unwrap();
        ctx.push().unwrap();
        ctx.assert(ctx.false_()).unwrap();
        // Different frame contents: this is a distinct cacheable point, not a
        // hit on the previous check.
        ctx.check().unwrap();
        ctx.exit().unwrap();

        let (hits, misses, stores, _) = cache.snapshot_stats();
        assert_eq!((hits, misses, stores), (0, 2, 2));
    }

    /// A cache miss in read-only-enforcing mode is an error and does not
    /// attempt to spawn a solver.
    #[test]
    fn test_read_only_enforcing_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Arc::new(Cache::open(
            Some(dir.path().to_path_buf()),
            None,
            CacheMode::ReadOnlyEnforcing,
        ));
        let mut ctx = unspawnable_context(Some(cache));
        ctx.set_logic("ALL").unwrap();
        let err = ctx.check().unwrap_err();
        assert!(err.to_string().contains("read-only-enforcing"));
    }

    /// Non-query commands never spawn a solver, even without a cache.
    #[test]
    fn test_no_spawn_without_query() {
        let mut ctx = unspawnable_context(None);
        ctx.set_logic("ALL").unwrap();
        let sort = ctx.declare_sort("S", 0).unwrap();
        let c = ctx.declare_const("c", sort).unwrap();
        ctx.assert(ctx.eq(c, c)).unwrap();
        ctx.push().unwrap();
        ctx.pop().unwrap();
        ctx.exit().unwrap();
    }

    /// Unbalanced pops are reported.
    #[test]
    fn test_unbalanced_pop() {
        let mut ctx = unspawnable_context(None);
        ctx.push().unwrap();
        ctx.pop().unwrap();
        assert!(ctx.pop().is_err());
    }
}
