//! gdbstub protocol implementation in Wasmtime's debug-main world.

mod addr;
mod api;
mod target;

use crate::{
    addr::AddrSpace,
    api::{WasmType, WasmValue},
};
use anyhow::Result;
use futures::{FutureExt, select};
use gdbstub::{
    common::{Signal, Tid},
    conn::Connection,
    stub::{
        MultiThreadStopReason,
        state_machine::{GdbStubStateMachine, GdbStubStateMachineInner, state::Running},
    },
};
use gdbstub_arch::wasm::addr::WasmAddr;
use log::trace;
use structopt::StructOpt;
use wstd::{
    io::{AsyncRead, AsyncWrite},
    iter::AsyncIterator,
    net::{TcpListener, TcpStream},
};

/// Command-line options.
#[derive(StructOpt)]
struct Options {
    /// The TCP address to listen on, in `<addr>:<port>` format.
    tcp_address: String,
    /// Verbose logging.
    #[structopt(short = "v")]
    verbose: bool,
}

struct Component;
api::export!(Component with_types_in api);

impl api::exports::bytecodealliance::wasmtime::debugger::Guest for Component {
    fn debug(d: &api::Debuggee, args: Vec<String>) {
        let options = Options::from_iter(args);
        if options.verbose {
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Trace)
                .init();
        }
        let mut debugger = Debugger {
            debuggee: d,
            tid: Tid::new(1).unwrap(),
            options,
            running: None,
            current_pc: WasmAddr::from_raw(0).unwrap(),
            interrupt: false,
            single_stepping: false,
            frame_cache: vec![],
            addr_space: AddrSpace::new(),
        };
        wstd::runtime::block_on(async {
            if let Err(e) = debugger.run().await {
                trace!("debugger exited with error: {e}");
            }
        });
    }
}

struct Debugger<'a> {
    debuggee: &'a api::Debuggee,
    tid: Tid,
    options: Options,
    running: Option<api::Resumption>,
    addr_space: AddrSpace,
    interrupt: bool,
    single_stepping: bool,
    current_pc: WasmAddr,
    frame_cache: Vec<api::Frame>,
}

impl<'a> Debugger<'a> {
    async fn run(&mut self) -> Result<()> {
        // Single-step once so modules are loaded and PC is at the
        // first instruction.
        self.start_single_step(api::ResumptionValue::Normal);
        self.running.as_mut().unwrap().wait().await;
        let _ = self.running.take().unwrap().result(self.debuggee)?;
        self.update_on_stop();

        let listener = TcpListener::bind(&self.options.tcp_address)
            .await
            .expect("Could not bind to TCP port");

        // Only accept one connection for the run; once the debugger
        // disconnects, we'll just continue.
        let Some(connection) = listener.incoming().next().await else {
            return Ok(());
        };

        let gdbconn = Conn::new(connection?);
        let mut stub = gdbstub::stub::GdbStub::new(gdbconn).run_state_machine(&mut *self)?;

        // Main loop.
        'mainloop: loop {
            match stub {
                GdbStubStateMachine::Idle(mut inner) => {
                    if inner.borrow_conn().flush().await.is_err() {
                        // Connection closed or other outbound error.
                        break 'mainloop;
                    }

                    // Wait for an inbound network byte.
                    let Some(byte) = inner.borrow_conn().read_byte().await? else {
                        inner.borrow_conn().flush().await?;
                        break 'mainloop;
                    };

                    stub = inner.incoming_data(self, byte)?;
                }

                GdbStubStateMachine::Running(mut inner) => {
                    if inner.borrow_conn().flush().await.is_err() {
                        // Connection closed or other outbound error.
                        break 'mainloop;
                    }

                    // Wait for either a resumption or a byte from the
                    // connection.
                    let resumption = self
                        .running
                        .as_mut()
                        .expect("In Running state, we must have a resumption future");
                    select! {
                        _ = resumption.wait().fuse() => {
                            let resumption = self.running.take().unwrap();
                            let event = resumption.result(self.debuggee)?;
                            stub = self.handle_event(event, inner).await?;
                        }
                        byte = inner.borrow_conn().read_byte().fuse() => {
                            let Some(byte) = byte? else {
                                // Eat any connection-closed errors on
                                // the outbound flush.
                                let _ = inner.borrow_conn().flush().await;
                                // Connection closed.
                                break 'mainloop;
                            };
                            stub = inner.incoming_data(&mut *self, byte)?;
                        }
                    }
                }
                GdbStubStateMachine::CtrlCInterrupt(mut inner) => {
                    if inner.borrow_conn().flush().await.is_err() {
                        // Connection error: break.
                        break 'mainloop;
                    }
                    stub = inner.interrupt_handled(self, None::<MultiThreadStopReason<u64>>)?;
                }
                GdbStubStateMachine::Disconnected(mut inner) => {
                    // Eat any connection-closed errors -- we are
                    // already in Disconnected state.
                    let _ = inner.borrow_conn().flush().await;
                    break 'mainloop;
                }
            }
        }

        Ok(())
    }

    fn start_continue(&mut self, resumption: api::ResumptionValue) {
        assert!(self.running.is_none());
        trace!("continuing");
        self.single_stepping = false;
        self.running = Some(api::Resumption::continue_(self.debuggee, resumption));
    }

    fn start_single_step(&mut self, resumption: api::ResumptionValue) {
        assert!(self.running.is_none());
        trace!("single-stepping");
        self.single_stepping = true;
        self.running = Some(api::Resumption::single_step(self.debuggee, resumption));
    }

    fn update_on_stop(&mut self) {
        self.addr_space.update(self.debuggee).unwrap();

        // Cache all frame handles for the duration of this stop.
        // The Wasm trait methods take `&self` and need access to
        // frames by depth, so we eagerly walk the full stack here.
        self.frame_cache.clear();
        let mut next = self.debuggee.exit_frames().into_iter().next();
        while let Some(f) = next {
            next = f.parent_frame(self.debuggee).unwrap();
            self.frame_cache.push(f);
        }

        if let Some(f) = self.frame_cache.first() {
            self.current_pc = self.addr_space.frame_to_pc(f, self.debuggee);
        } else {
            self.current_pc = WasmAddr::from_raw(0).unwrap();
        }
    }

    async fn handle_event<'b>(
        &mut self,
        event: api::Event,
        inner: GdbStubStateMachineInner<'b, Running, Self, Conn>,
    ) -> Result<GdbStubStateMachine<'b, Self, Conn>> {
        match event {
            api::Event::Complete => {
                trace!("Event::Complete");
                let pc_bytes = self.current_pc.as_raw().to_le_bytes();
                let mut regs = core::iter::once((
                    gdbstub_arch::wasm::reg::id::WasmRegId::Pc,
                    pc_bytes.as_slice(),
                ));
                Ok(inner.report_stop_with_regs(
                    self,
                    MultiThreadStopReason::Exited(0),
                    &mut regs,
                )?)
            }
            api::Event::Breakpoint => {
                trace!(
                    "Event::Breakpoint; single_stepping = {}",
                    self.single_stepping
                );
                self.update_on_stop();
                let stop_reason = if self.single_stepping {
                    MultiThreadStopReason::SignalWithThread {
                        tid: self.tid,
                        signal: Signal::SIGTRAP,
                    }
                } else {
                    MultiThreadStopReason::SwBreak(self.tid)
                };
                let pc_bytes = self.current_pc.as_raw().to_le_bytes();
                let mut regs = core::iter::once((
                    gdbstub_arch::wasm::reg::id::WasmRegId::Pc,
                    pc_bytes.as_slice(),
                ));
                Ok(inner.report_stop_with_regs(self, stop_reason, &mut regs)?)
            }
            _ => {
                trace!("other event: {event:?}");
                if self.interrupt {
                    self.interrupt = false;
                    self.update_on_stop();
                    let pc_bytes = self.current_pc.as_raw().to_le_bytes();
                    let mut regs = core::iter::once((
                        gdbstub_arch::wasm::reg::id::WasmRegId::Pc,
                        pc_bytes.as_slice(),
                    ));
                    Ok(inner.report_stop_with_regs(
                        self,
                        MultiThreadStopReason::Signal(Signal::SIGINT),
                        &mut regs,
                    )?)
                } else {
                    if self.single_stepping {
                        self.start_single_step(api::ResumptionValue::Normal);
                    } else {
                        self.start_continue(api::ResumptionValue::Normal);
                    }
                    Ok(GdbStubStateMachine::Running(inner))
                }
            }
        }
    }

    fn value_to_bytes(&self, value: WasmValue) -> Vec<u8> {
        match value.get_type() {
            WasmType::WasmI32 => value.unwrap_i32().to_le_bytes().to_vec(),
            WasmType::WasmI64 => value.unwrap_i64().to_le_bytes().to_vec(),
            WasmType::WasmF32 => value.unwrap_f32().to_le_bytes().to_vec(),
            WasmType::WasmF64 => value.unwrap_f64().to_le_bytes().to_vec(),
            WasmType::WasmV128 => value.unwrap_v128(),
            WasmType::WasmFuncref => 0u32.to_le_bytes().to_vec(),
            WasmType::WasmExnref => 0u32.to_le_bytes().to_vec(),
        }
    }
}

struct Conn {
    buf: Vec<u8>,
    conn: TcpStream,
}

impl Conn {
    fn new(conn: TcpStream) -> Self {
        Conn { buf: vec![], conn }
    }

    async fn flush(&mut self) -> anyhow::Result<()> {
        self.conn.write_all(&self.buf).await?;
        self.buf.clear();
        Ok(())
    }

    async fn read_byte(&mut self) -> Result<Option<u8>> {
        let mut buf = [0u8];
        let len = self.conn.read(&mut buf).await?;
        if len == 1 { Ok(Some(buf[0])) } else { Ok(None) }
    }
}

impl Drop for Conn {
    fn drop(&mut self) {
        assert!(
            self.buf.is_empty(),
            "failed to async-flush before dropping connection write buffer"
        );
    }
}

impl Connection for Conn {
    type Error = anyhow::Error;

    fn write(&mut self, byte: u8) -> std::result::Result<(), Self::Error> {
        self.buf.push(byte);
        Ok(())
    }

    fn flush(&mut self) -> std::result::Result<(), Self::Error> {
        // We cannot flush synchronously; we leave this to the `async
        // fn flush` method called within the main loop. Fortunately
        // the gdbstub cannot wait for a response before returning to
        // the main loop, so we cannot introduce any deadlocks by
        // failing to flush synchronously here.
        Ok(())
    }
}
