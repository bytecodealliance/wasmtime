//! gdbstub `Target` implementation.

use crate::Debugger;
use crate::addr::AddrSpaceLookup;
use crate::api;
use gdbstub::arch::lldb::{Encoding, Format, Generic, Register};
use gdbstub::common::{Endianness, Pid, Signal, Tid};
use gdbstub::target::Target;
use gdbstub::target::TargetError;
use gdbstub::target::TargetResult;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::multithread::{
    MultiThreadBase, MultiThreadResume, MultiThreadResumeOps, MultiThreadSchedulerLocking,
    MultiThreadSchedulerLockingOps, MultiThreadSingleStep, MultiThreadSingleStepOps,
};
use gdbstub::target::ext::base::single_register_access::{
    SingleRegisterAccess, SingleRegisterAccessOps,
};
use gdbstub::target::ext::breakpoints::{
    Breakpoints, BreakpointsOps, SwBreakpoint, SwBreakpointOps,
};
use gdbstub::target::ext::host_info::{HostInfo, HostInfoOps, HostInfoResponse};
use gdbstub::target::ext::libraries::{Libraries, LibrariesOps};
use gdbstub::target::ext::lldb_register_info_override::{
    Callback, CallbackToken, LldbRegisterInfoOverride, LldbRegisterInfoOverrideOps,
};
use gdbstub::target::ext::memory_map::{MemoryMap, MemoryMapOps};
use gdbstub::target::ext::process_info::{ProcessInfo, ProcessInfoOps, ProcessInfoResponse};
use gdbstub::target::ext::wasm::{Wasm, WasmOps};
use gdbstub_arch::wasm::Wasm as WasmArch;
use gdbstub_arch::wasm::addr::WasmAddr;
use gdbstub_arch::wasm::reg::WasmRegisters;
use gdbstub_arch::wasm::reg::id::WasmRegId;

impl<'a> Target for Debugger<'a> {
    type Arch = WasmArch;
    type Error = anyhow::Error;

    fn base_ops(&mut self) -> BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::MultiThread(self)
    }

    fn support_wasm(&mut self) -> Option<WasmOps<'_, Self>> {
        Some(self)
    }

    fn use_lldb_register_info(&self) -> bool {
        true
    }

    fn support_lldb_register_info_override(
        &mut self,
    ) -> Option<LldbRegisterInfoOverrideOps<'_, Self>> {
        Some(self)
    }

    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<'_, Self>> {
        Some(self)
    }

    fn support_libraries(&mut self) -> Option<LibrariesOps<'_, Self>> {
        Some(self)
    }

    fn support_memory_map(&mut self) -> Option<MemoryMapOps<'_, Self>> {
        Some(self)
    }

    fn support_process_info(&mut self) -> Option<ProcessInfoOps<'_, Self>> {
        Some(self)
    }

    fn support_host_info(&mut self) -> Option<HostInfoOps<'_, Self>> {
        Some(self)
    }
}

impl<'a> MultiThreadBase for Debugger<'a> {
    fn read_registers(&mut self, regs: &mut WasmRegisters, _tid: Tid) -> TargetResult<(), Self> {
        regs.pc = self.current_pc.as_raw();
        Ok(())
    }

    fn write_registers(&mut self, regs: &WasmRegisters, _tid: Tid) -> TargetResult<(), Self> {
        self.current_pc = WasmAddr::from_raw(regs.pc).ok_or(TargetError::NonFatal)?;
        Ok(())
    }

    fn read_addrs(
        &mut self,
        start_addr: u64,
        data: &mut [u8],
        _tid: Tid,
    ) -> TargetResult<usize, Self> {
        let addr = WasmAddr::from_raw(start_addr).ok_or(TargetError::NonFatal)?;
        let debuggee = self.debuggee;
        match self.addr_space.lookup(addr, debuggee) {
            AddrSpaceLookup::Module {
                bytecode, offset, ..
            } => {
                let offset = usize::try_from(offset).unwrap();
                let avail = bytecode.len() - offset;
                let n = avail.min(data.len());
                data[..n].copy_from_slice(&bytecode[offset..offset + n]);
                Ok(n)
            }
            AddrSpaceLookup::Memory { memory, offset } => {
                match memory.get_bytes(debuggee, offset.into(), u64::try_from(data.len()).unwrap())
                {
                    Ok(bytes) => {
                        assert_eq!(bytes.len(), data.len());
                        data.copy_from_slice(&bytes);
                        Ok(data.len())
                    }
                    Err(_) => Err(TargetError::NonFatal),
                }
            }
            AddrSpaceLookup::Empty => Err(TargetError::NonFatal),
        }
    }

    fn write_addrs(&mut self, _start_addr: u64, _data: &[u8], _tid: Tid) -> TargetResult<(), Self> {
        Err(TargetError::NonFatal)
    }

    #[inline(always)]
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        thread_is_active(self.tid);
        Ok(())
    }

    fn support_single_register_access(&mut self) -> Option<SingleRegisterAccessOps<'_, Tid, Self>> {
        Some(self)
    }

    fn support_resume(&mut self) -> Option<MultiThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

impl<'a> SingleRegisterAccess<Tid> for Debugger<'a> {
    fn read_register(
        &mut self,
        _tid: Tid,
        reg_id: WasmRegId,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        match reg_id {
            WasmRegId::Pc => {
                let bytes = self.current_pc.as_raw().to_le_bytes();
                let n = bytes.len().min(buf.len());
                buf[..n].copy_from_slice(&bytes[..n]);
                Ok(n)
            }
            _ => Err(TargetError::NonFatal),
        }
    }

    fn write_register(
        &mut self,
        _tid: Tid,
        reg_id: WasmRegId,
        val: &[u8],
    ) -> TargetResult<(), Self> {
        match reg_id {
            WasmRegId::Pc => {
                if val.len() < 8 {
                    return Err(TargetError::NonFatal);
                }
                let raw = u64::from_le_bytes(val[..8].try_into().unwrap());
                self.current_pc = WasmAddr::from_raw(raw).ok_or(TargetError::NonFatal)?;
                Ok(())
            }
            _ => Err(TargetError::NonFatal),
        }
    }
}

impl<'a> MultiThreadResume for Debugger<'a> {
    fn resume(&mut self) -> Result<(), Self::Error> {
        self.frame_cache.clear();
        log::trace!("resume() -> single_stepping = {}", self.single_stepping);
        if self.single_stepping {
            self.start_single_step(api::ResumptionValue::Normal);
        } else {
            self.start_continue(api::ResumptionValue::Normal);
        }
        Ok(())
    }

    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        self.single_stepping = false;
        Ok(())
    }

    fn set_resume_action_continue(
        &mut self,
        _tid: Tid,
        _signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        self.single_stepping = false;
        Ok(())
    }

    fn support_single_step(&mut self) -> Option<MultiThreadSingleStepOps<'_, Self>> {
        Some(self)
    }

    fn support_scheduler_locking(&mut self) -> Option<MultiThreadSchedulerLockingOps<'_, Self>> {
        Some(self)
    }
}

impl<'a> MultiThreadSingleStep for Debugger<'a> {
    fn set_resume_action_step(
        &mut self,
        _tid: Tid,
        _signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        self.single_stepping = true;
        Ok(())
    }
}

impl<'a> MultiThreadSchedulerLocking for Debugger<'a> {
    fn set_resume_action_scheduler_lock(&mut self) -> Result<(), Self::Error> {
        // We have a single thread, so scheduler locking is a no-op.
        Ok(())
    }
}

impl<'a> Breakpoints for Debugger<'a> {
    #[inline(always)]
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl<'a> SwBreakpoint for Debugger<'a> {
    fn add_sw_breakpoint(&mut self, addr: u64, _kind: usize) -> TargetResult<bool, Self> {
        let Some(wasm_addr) = WasmAddr::from_raw(addr) else {
            return Ok(false);
        };
        let debuggee = self.debuggee;
        if let AddrSpaceLookup::Module { module, .. } = self.addr_space.lookup(wasm_addr, debuggee)
        {
            module
                .add_breakpoint(debuggee, wasm_addr.offset())
                .map_err(|_| TargetError::NonFatal)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn remove_sw_breakpoint(&mut self, addr: u64, _kind: usize) -> TargetResult<bool, Self> {
        let Some(wasm_addr) = WasmAddr::from_raw(addr) else {
            return Ok(false);
        };
        let debuggee = self.debuggee;
        if let AddrSpaceLookup::Module { module, .. } = self.addr_space.lookup(wasm_addr, debuggee)
        {
            module
                .remove_breakpoint(debuggee, wasm_addr.offset())
                .map_err(|_| TargetError::NonFatal)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<'a> LldbRegisterInfoOverride for Debugger<'a> {
    fn lldb_register_info<'b>(
        &mut self,
        reg_id: usize,
        reg_info: Callback<'b>,
    ) -> Result<CallbackToken<'b>, Self::Error> {
        Ok(match reg_id {
            0 => reg_info.write(Register {
                name: "pc",
                alt_name: Some("pc"),
                bitsize: 64,
                offset: 0,
                encoding: Encoding::Uint,
                format: Format::Hex,
                set: "PC",
                gcc: Some(16),
                dwarf: Some(16),
                generic: Some(Generic::Pc),
                container_regs: None,
                invalidate_regs: None,
            }),
            _ => reg_info.done(),
        })
    }
}

impl<'a> Libraries for Debugger<'a> {
    fn get_libraries(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let mut xml = String::from("<library-list>");
        for addr in self.addr_space.module_base_addrs() {
            xml.push_str(&format!(
                "<library name=\"wasm\"><section address=\"{}\"/></library>",
                addr.as_raw()
            ));
        }
        xml.push_str("</library-list>");

        let xml_bytes = xml.as_bytes();
        let offset = usize::try_from(offset).unwrap();
        if offset >= xml_bytes.len() {
            return Ok(0);
        }
        let avail = xml_bytes.len() - offset;
        let n = avail.min(length).min(buf.len());
        buf[..n].copy_from_slice(&xml_bytes[offset..offset + n]);
        Ok(n)
    }
}

impl<'a> MemoryMap for Debugger<'a> {
    fn memory_map_xml(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let xml = self.addr_space.memory_map_xml(self.debuggee);
        let xml_bytes = xml.as_bytes();
        let offset = usize::try_from(offset).unwrap();
        if offset >= xml_bytes.len() {
            return Ok(0);
        }
        let avail = xml_bytes.len() - offset;
        let n = avail.min(length).min(buf.len());
        buf[..n].copy_from_slice(&xml_bytes[offset..offset + n]);
        Ok(n)
    }
}

impl<'a> Wasm for Debugger<'a> {
    fn wasm_call_stack(&self, _tid: Tid, callback: &mut dyn FnMut(u64)) -> Result<(), Self::Error> {
        let debuggee = self.debuggee;
        for (i, f) in self.frame_cache.iter().enumerate() {
            // For non-innermost frames, report the return address
            // (the instruction after the call) rather than the call
            // instruction's PC. This matches the standard debugger
            // convention and is needed for LLDB's `finish` command
            // to set a breakpoint at the right address.
            let pc = if i > 0 {
                self.addr_space
                    .frame_to_return_addr(f, debuggee)
                    .unwrap_or_else(|| self.addr_space.frame_to_pc(f, debuggee))
            } else {
                self.addr_space.frame_to_pc(f, debuggee)
            };
            callback(pc.as_raw());
        }
        Ok(())
    }

    fn read_wasm_local(
        &self,
        _tid: Tid,
        frame_depth: usize,
        index: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let Some(f) = self.frame_cache.get(frame_depth) else {
            return Ok(0);
        };
        let Ok(locals) = f.get_locals(self.debuggee) else {
            return Ok(0);
        };
        let Some(val) = locals.get(index) else {
            return Ok(0);
        };
        let bytes = self.value_to_bytes(val.clone());
        buf[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }

    fn read_wasm_global(
        &self,
        _tid: Tid,
        frame_depth: usize,
        index: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let Some(f) = self.frame_cache.get(frame_depth) else {
            return Ok(0);
        };
        let debuggee = self.debuggee;
        let Ok(instance) = f.get_instance(debuggee) else {
            return Ok(0);
        };
        let Ok(global) = instance.get_global(debuggee, u32::try_from(index).unwrap()) else {
            return Ok(0);
        };
        let Ok(val) = global.get(debuggee) else {
            return Ok(0);
        };
        let bytes = self.value_to_bytes(val);
        buf[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }

    fn read_wasm_stack(
        &self,
        _tid: Tid,
        frame_depth: usize,
        index: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let Some(f) = self.frame_cache.get(frame_depth) else {
            return Ok(0);
        };
        let Ok(stack) = f.get_stack(self.debuggee) else {
            return Ok(0);
        };
        let Some(val) = stack.get(index) else {
            return Ok(0);
        };
        let bytes = self.value_to_bytes(val.clone());
        buf[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

impl<'a> HostInfo for Debugger<'a> {
    fn host_info(
        &self,
        write_item: &mut dyn FnMut(&HostInfoResponse<'_>),
    ) -> Result<(), Self::Error> {
        write_item(&HostInfoResponse::Triple("wasm32-unknown-unknown-wasm"));
        write_item(&HostInfoResponse::Endianness(Endianness::Little));
        write_item(&HostInfoResponse::PointerSize(4));
        Ok(())
    }
}

impl<'a> ProcessInfo for Debugger<'a> {
    fn process_info(
        &self,
        write_item: &mut dyn FnMut(&ProcessInfoResponse<'_>),
    ) -> Result<(), Self::Error> {
        write_item(&ProcessInfoResponse::Pid(Pid::new(1).unwrap()));
        write_item(&ProcessInfoResponse::Triple("wasm32-unknown-unknown-wasm"));
        write_item(&ProcessInfoResponse::Endianness(Endianness::Little));
        write_item(&ProcessInfoResponse::PointerSize(4));
        Ok(())
    }
}
