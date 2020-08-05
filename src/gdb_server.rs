#![allow(dead_code, unused_variables, unused_imports)]

use anyhow::Result;
use gdb_remote_protocol::*;
use log::{error, trace};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::ops::Range;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use wasmtime::debugger::{
    DebuggerAgent, DebuggerJitCodeRegistration, DebuggerModule, DebuggerPauseKind,
    DebuggerResumeAction,
};
use wasmtime::{Config, Trap};

fn hex_byte_sequence(s: &str) -> String {
    s.as_bytes().iter().fold(String::new(), |mut acc, ch| {
        acc.extend(format!("{:02x}", ch).chars());
        acc
    })
}

struct DebuggerHandler {
    modules: Vec<(u64, String, Vec<u8>)>,
    tx: Sender<()>,
}

impl Handler for DebuggerHandler {
    fn query_supported_features(&self) -> Vec<String> {
        vec![
            "PacketSize=1000".to_string(),
            "vContSupported-".to_string(),
            "qXfer:libraries:read+".to_string(),
        ]
    }

    fn attached(&self, _: Option<u64>) -> Result<ProcessType, Error> {
        Ok(ProcessType::Attached)
    }

    fn register_info(&self, reg: u64) -> Result<String, Error> {
        match reg {
            0 => Ok("name:pc;alt-name:pc;bitsize:64;offset:0;encoding:uint;format:hex;set:General Purpose Registers;gcc:16;dwarf:16;generic:pc;".to_string()),
            _ => Err(Error::Error(0x45)),
        }
    }

    fn process_info(&self) -> Result<String, Error> {
        Ok(format!(
            "pid:1;ppid:1;uid:1;gid:1;euid:1;egid:1;name:6c6c6462;triple:{};ptrsize:4;",
            hex_byte_sequence("wasm32-unknown-unknown-wasm")
        ))
    }

    fn process_symbol(
        &self,
        sym_value: &str,
        sym_name: &str,
    ) -> Result<SymbolLookupResponse, Error> {
        Ok(SymbolLookupResponse::Ok)
    }

    fn thread_list(&self, reset: bool) -> Result<Vec<ThreadId>, Error> {
        Ok(if reset {
            vec![ThreadId {
                pid: Id::Id(1),
                tid: Id::Id(1),
            }]
        } else {
            vec![]
        })
    }

    fn read_general_registers(&self) -> Result<Vec<u8>, Error> {
        Ok(vec![0x6B, 0, 0, 0, 1, 0, 0, 0]) // TODO pc
    }

    fn current_thread(&self) -> Result<Option<ThreadId>, Error> {
        Ok(Some(ThreadId {
            pid: Id::Id(1),
            tid: Id::Id(1),
        }))
    }

    fn read_bytes(
        &self,
        object: String,
        annex: String,
        _offset: u64,
        _length: u64,
    ) -> Result<(Vec<u8>, bool), Error> {
        if object == "libraries" && annex.is_empty() {
            let libs =
                self.modules
                    .iter()
                    .fold("<library-list>".to_string(), |s, (addr, name, _)| {
                        format!(
                            r#"{}\n<library name="{}"><section address="0x{:08x}"/></library>"#,
                            s, name, addr
                        )
                    });
            return Ok((format!("{}</library-list>", libs).as_bytes().to_vec(), true));
        }
        Err(Error::Unimplemented)
    }

    fn read_register(&self, register: u64) -> Result<Vec<u8>, Error> {
        if register == 0 {
            Ok(vec![0x6B, 0, 0, 0, 1, 0, 0, 0]) // TODO pcs
        } else {
            Err(Error::Unimplemented)
        }
    }

    fn halt_reason(
        &self,
    ) -> std::result::Result<gdb_remote_protocol::StopReason, gdb_remote_protocol::Error> {
        Ok(StopReason::Signal(0)) // TODO real signal
    }

    fn process_continue(&self) -> Result<(), Error> {
        self.tx.send(()).unwrap();
        Ok(())
    }

    fn read_memory(&self, region: MemoryRegion) -> Result<Vec<u8>, Error> {
        let lib = self
            .modules
            .iter()
            .find(|m| m.0 <= region.address && region.address < m.0 + m.2.len() as u64);
        match lib {
            Some(lib) => {
                let chunk = &lib.2[(region.address - lib.0) as usize..];
                Ok((if (chunk.len() as u64) < region.length {
                    chunk
                } else {
                    &chunk[..region.length as usize]
                })
                .to_vec())
            }
            None => Err(Error::Error(0x01)),
        }
    }

    fn insert_software_breakpoint(&self, breakpoint: Breakpoint) -> Result<(), Error> {
        trace!("Breakpoint {:x}", breakpoint.addr);
        Ok(())
    }

    fn wasm_call_stack(&self) -> Result<Vec<u8>, Error> {
        Ok(vec![0x6B, 0, 0, 0, 1, 0, 0, 0]) // TODO pcs
    }

    fn wasm_local(&self, frame: u64, index: u64) -> Result<Vec<u8>, Error> {
        trace!("Local {} at {}", index, frame);
        Ok(vec![0, 0, 0, 0, 0, 0, 0, 0]) // TODO local
    }

    fn wasm_memory(&self, frame: u64, addr: u64, len: u64) -> Result<Vec<u8>, Error> {
        trace!("Memory {:x}/{:x} at {}", addr, len, frame);
        Ok(vec![0; len as usize]) // TODO memory
    }
}

fn handle_client(stream: TcpStream, tx: Sender<()>) {
    let h = DebuggerHandler {
        // modules: vec![(
        //     0x1_0000_0000,
        //     "fib.wasm".to_string(),
        //     read("fib.wasm").expect("fib module"),
        // )],
        modules: vec![(0, "".to_string(), WAIT_WASM.to_vec())],
        tx,
    };
    process_packets_from(
        stream.try_clone().expect("TCPStream::try_clone failed!"),
        stream,
        h,
    );
    // TODO? stream.shutdown(Shutdown::Both).unwrap();
}

pub(crate) struct GdbServer {
    handler: thread::JoinHandle<()>,
    count: u32,
    connection_expected: bool,
    rx: Mutex<Receiver<()>>,
}

impl DebuggerAgent for GdbServer {
    fn pause(&mut self, kind: DebuggerPauseKind) -> DebuggerResumeAction {
        match kind {
            DebuggerPauseKind::Breakpoint(_) => {
                if self.connection_expected {
                    trace!("Waiting for debugger connection");
                    let _ = self.rx.lock().unwrap().recv().unwrap();
                    self.connection_expected = false;
                    trace!("Debugger connected");
                    return DebuggerResumeAction::Continue;
                }
                println!("!brk");
                DebuggerResumeAction::Step
            }
            DebuggerPauseKind::Step => {
                if self.count > 0 {
                    self.count -= 1;
                    let t = Trap::new("test");
                    println!("!step {:?}", t.trace());
                    DebuggerResumeAction::Step
                } else {
                    DebuggerResumeAction::Continue
                }
            }
        }
    }

    fn register_module(&mut self, _module: DebuggerModule) -> Box<dyn DebuggerJitCodeRegistration> {
        struct NullReg;
        impl DebuggerJitCodeRegistration for NullReg {}
        Box::new(NullReg)
    }
}

impl GdbServer {
    pub(crate) fn new(port: u16) -> Self {
        let (tx, rx) = channel();
        let listener = TcpListener::bind(&format!("0.0.0.0:{}", port)).unwrap();
        trace!("Server listening on port {}", port);
        let handler = thread::spawn(move || {
            for stream in listener.incoming() {
                let tx = tx.clone();
                match stream {
                    Ok(stream) => {
                        trace!("New connection: {}", stream.peer_addr().unwrap());
                        thread::spawn(move || handle_client(stream, tx));
                    }
                    Err(e) => {
                        error!("Error: {}", e);
                    }
                }
            }
        });
        Self {
            handler,
            count: 5,
            connection_expected: true,
            rx: Mutex::new(rx),
        }
    }
}

const WAIT_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x02, 0x06,
    0x01, 0x00, 0x01, 0x63, 0x00, 0x00, 0x03, 0x02, 0x01, 0x00, 0x08, 0x01, 0x01, 0x0a, 0x06, 0x01,
    0x04, 0x00, 0x10, 0x00, 0x0b,
];
const FUNC_CALL_OFFSET: usize = WAIT_WASM.len() - 3;

pub fn wait_for_debugger_connection(config: &Config) -> Result<()> {
    use wasmtime::*;
    let engine = Engine::new(&config);
    let store = Store::new(&engine);

    // (module (func nop) (start 0))
    let module = Module::from_binary(&engine, WAIT_WASM)?;
    module.set_breakpoint(FUNC_CALL_OFFSET);
    let func = Func::wrap(&store, || {
        trace!("wait_for_debugger_connection callback");
    });
    let _instance = Instance::new(&store, &module, &[func.into()])?;
    Ok(())
}
