use gdbstub::target::{Target, TargetResult, TargetError};
use gdbstub_arch::x86::X86_64_SSE;
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::multithread::{MultiThreadBase, MultiThreadResume, MultiThreadSingleStep};
use gdbstub::target::ext::base::multithread::{MultiThreadResumeOps, MultiThreadSingleStepOps};
use gdbstub::common::{Tid};
use gdbstub::arch::Arch;
use crate::{scheduler, process_manager};
use x86_64::VirtAddr;
use gdbstub::target::ext::breakpoints::{Breakpoints, SwBreakpoint};
use gdbstub::target::ext::breakpoints::{BreakpointsOps, SwBreakpointOps};
use crate::process::process::Process;
use alloc::sync::Arc;
use log::info;
use volatile::Volatile;
use x86_64::structures::idt::InterruptStackFrame;
use spin::Mutex;
use alloc::vec::Vec;
use crate::gdbstub::debug_state::{GDB_DEBUG_STATE, DebugEvent};
use gdbstub::stub::MultiThreadStopReason;
use crate::device::cpu::{disable_int_nested};


pub struct GdbStubTarget {
    selected_pid: Arc<Process>,
    breakpoints: Mutex<Vec<GdbSwBreakpoint>>,
}

impl GdbStubTarget {
    pub fn new() -> Self {
        let selected_pid = process_manager().read().kernel_process().unwrap();

        Self {
            selected_pid,
            breakpoints: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GdbSwBreakpoint {
    address: u64,
    instruction: u8,
}

const THREAD_REG_COUNT: usize = 19;

#[repr(u64)]
#[derive(Clone, Copy, Debug)]
pub enum ThreadRegs {
    Gsbase = 0,
    Fsbase = 1,
    Rbp = 2,
    Rdi = 3,
    Rsi = 4,
    Rdx = 5,
    Rcx = 6,
    Rbx = 7,
    Rax = 8,
    R15 = 9,
    R14 = 10,
    R13 = 11,
    R12 = 12,
    R11 = 13,
    R10 = 14,
    R9 = 15,
    R8 = 16,
    Rflags = 17,
    Rip = 18,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadContext {
    pub registers: [u64; THREAD_REG_COUNT],
    pub rsp: u64,
}

struct ThreadContextMut<'a> {
    registers: &'a mut [u64; THREAD_REG_COUNT],
    rsp: u64,
}

impl From<ThreadContext> for X86_64CoreRegs {
    fn from(ctx: ThreadContext) -> Self {
        let mut regs = X86_64CoreRegs::default();
        regs.regs[0] = ctx.registers[ThreadRegs::Rax as usize];
        regs.regs[1] = ctx.registers[ThreadRegs::Rbx as usize];
        regs.regs[2] = ctx.registers[ThreadRegs::Rcx as usize];
        regs.regs[3] = ctx.registers[ThreadRegs::Rdx as usize];
        regs.regs[4] = ctx.registers[ThreadRegs::Rsi as usize];
        regs.regs[5] = ctx.registers[ThreadRegs::Rdi as usize];
        regs.regs[6] = ctx.registers[ThreadRegs::Rbp as usize];
        regs.regs[7] = ctx.rsp; 
        regs.regs[8] = ctx.registers[ThreadRegs::R8 as usize];
        regs.regs[9] = ctx.registers[ThreadRegs::R9 as usize];
        regs.regs[10] = ctx.registers[ThreadRegs::R10 as usize];
        regs.regs[11] = ctx.registers[ThreadRegs::R11 as usize];
        regs.regs[12] = ctx.registers[ThreadRegs::R12 as usize];
        regs.regs[13] = ctx.registers[ThreadRegs::R13 as usize];
        regs.regs[14] = ctx.registers[ThreadRegs::R14 as usize];
        regs.regs[15] = ctx.registers[ThreadRegs::R15 as usize];
        regs.rip = ctx.registers[ThreadRegs::Rip as usize];
        regs.eflags = ctx.registers[ThreadRegs::Rflags as usize] as u32;

        regs
    }
}

impl Target for GdbStubTarget {
    type Error = ();
    type Arch = X86_64_SSE;

    #[inline(always)]
    fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
        BaseOps::MultiThread(self)
    }

    #[inline(always)]
    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<Self>> {
        Some(self)
    }
}

impl MultiThreadBase for GdbStubTarget {
    fn read_registers(
        &mut self,
        regs: &mut X86_64CoreRegs,
        tid: Tid
    ) -> TargetResult<(), Self> {
        let raw_tid = tid.get();
        let thread = scheduler()
                    .thread(tid.get())
                    .ok_or(TargetError::NonFatal)?;
        info!("gdb tid={} -> thread.id={} saved_rsp0={:#x}", raw_tid, thread.id(), thread.saved_rsp0().as_u64());

        let rsp0 = thread.saved_rsp0();
        let ctx = thread_context_from_rsp(rsp0).ok_or(TargetError::NonFatal)?;
        *regs = X86_64CoreRegs::from(ctx);

        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &X86_64CoreRegs,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        let thread = scheduler()
                    .thread(tid.get())
                    .ok_or(TargetError::NonFatal)?;
        
        let rsp0 = thread.saved_rsp0();
        let mut ctx = mut_thread_context_from_rsp(rsp0).ok_or(TargetError::NonFatal)?;
        ctx.registers[ThreadRegs::Rax as usize] = regs.regs[0];
        ctx.registers[ThreadRegs::Rbx as usize] = regs.regs[1];
        ctx.registers[ThreadRegs::Rcx as usize] = regs.regs[2];
        ctx.registers[ThreadRegs::Rdx as usize] = regs.regs[3];
        ctx.registers[ThreadRegs::Rsi as usize] = regs.regs[4];
        ctx.registers[ThreadRegs::Rdi as usize] = regs.regs[5];
        ctx.registers[ThreadRegs::Rbp as usize] = regs.regs[6];
        ctx.rsp = regs.regs[7];
        ctx.registers[ThreadRegs::R8 as usize] = regs.regs[8];
        ctx.registers[ThreadRegs::R9 as usize] = regs.regs[9];
        ctx.registers[ThreadRegs::R10 as usize] = regs.regs[10];
        ctx.registers[ThreadRegs::R11 as usize] = regs.regs[11];
        ctx.registers[ThreadRegs::R12 as usize] = regs.regs[12];
        ctx.registers[ThreadRegs::R13 as usize] = regs.regs[13];
        ctx.registers[ThreadRegs::R14 as usize] = regs.regs[14];
        ctx.registers[ThreadRegs::R15 as usize] = regs.regs[15];
        ctx.registers[ThreadRegs::Rip as usize] = regs.rip;
        ctx.registers[ThreadRegs::Rflags as usize] = regs.eflags as u64;

        Ok(())
    }
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
        tid: Tid
    ) -> TargetResult<usize, Self> {
        let thread = scheduler()
                    .thread(tid.get())
                    .ok_or(TargetError::NonFatal)?;

        let process = thread.process();

        for (offset, byte) in data.iter_mut().enumerate() {
            let virt_addr = start_addr + offset as u64;

            let phys_addr = process
                            .virtual_address_space
                            .get_phys(virt_addr)
                            .ok_or(TargetError::NonFatal)?;

            unsafe { *byte = *(phys_addr.as_u64() as *const u8); }
        }

        Ok(data.len())
    }
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
        tid: Tid
    ) -> TargetResult<(), Self> {
        let thread = scheduler()
                    .thread(tid.get())
                    .ok_or(TargetError::NonFatal)?;

        let process = thread.process();

        for (offset, byte) in data.iter().enumerate() {
            let virt_addr = start_addr + offset as u64;

            let phys_addr = process
                            .virtual_address_space
                            .get_phys(virt_addr)
                            .ok_or(TargetError::NonFatal)?;

            unsafe { *(phys_addr.as_u64() as *mut u8) = *byte; }
        }


        Ok(())
    }
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid)
    ) -> Result<(), Self::Error> {
        let active_ids = scheduler().active_thread_ids();

        for id in active_ids {
            if id != 0 {
                thread_is_active(Tid::new(id).unwrap());
            }
        }
        Ok(())
    }
}

impl Breakpoints for GdbStubTarget {
    #[inline(always)]
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<Self>> {
        Some(self)
    }
}

impl SwBreakpoint for GdbStubTarget {
    fn add_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        let virt_addr = addr as u64;
        let virt_addr_ptr = virt_addr as *mut u8;
        let instruction = unsafe { virt_addr_ptr.read_volatile() };
        let mut breakpoints = self.breakpoints.lock();

        if breakpoints.iter().any(|bp| bp.address == virt_addr) {
            return Ok(true);
        }

        unsafe { virt_addr_ptr.write_volatile(0xCC) };
        unsafe { info!("OP CODE AT BREAKPOINT={:#x}", virt_addr_ptr.read_volatile()) };
        breakpoints.push(GdbSwBreakpoint{address: virt_addr, instruction});

        Ok(true)
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        let virt_addr = addr as u64;
        let virt_addr_ptr = unsafe { virt_addr as *mut u8 };
        let mut breakpoints = self.breakpoints.lock();

        let Some(index) = breakpoints.iter().position(|bp| bp.address == virt_addr) else {
            return Ok(false);
        };

        let bp = breakpoints.remove(index);
        let instruction = bp.instruction;
        unsafe { virt_addr_ptr.write_volatile(instruction) };

        Ok(true)
    }
}

pub fn handle_interrupt(frame: InterruptStackFrame, index: u8, error: Option<u64>) {
    disable_int_nested();
    let rip_after_int3 = frame.instruction_pointer.as_u64();
    let bp_addr = rip_after_int3 - 1;

    let tid = scheduler().current_thread().id();
    let mut state = GDB_DEBUG_STATE.lock();

    if state.ctrlc_pending {
        state.ctrlc_pending = false;
        state.event = Some(DebugEvent::CtrlC);
        info!("GDB CTRL-C at RIP={:#x}---------------------------------------------------------------------------------------------------------------", rip_after_int3);
        return;
    }

    state.event = Some(DebugEvent::SwBreakpoint {
        tid,
        addr: bp_addr,
    });

    info!("BREAKPOINT AT {:#x}", bp_addr);
}

pub fn thread_context_from_rsp(rsp: VirtAddr) -> Option<ThreadContext> {
    if rsp.is_null() {
        return None;
    }
    let raw_rsp0 = rsp.as_u64();

    let registers = unsafe { (raw_rsp0 as *const [u64; THREAD_REG_COUNT]).read() };
    let restored_rsp = raw_rsp0 + (THREAD_REG_COUNT * core::mem::size_of::<u64>()) as u64;

    Some(ThreadContext {
        registers,
        rsp: restored_rsp,
    })
}

pub fn mut_thread_context_from_rsp(rsp: VirtAddr) -> Option<ThreadContextMut<'static>> {
    if rsp.is_null() {
        return None;
    }

    let raw_rsp0 = rsp.as_u64();

    let registers = unsafe { &mut *(raw_rsp0 as *mut [u64; THREAD_REG_COUNT]) };
    let restored_rsp = raw_rsp0 + (THREAD_REG_COUNT * core::mem::size_of::<u64>()) as u64;



    Some(ThreadContextMut {
        registers,
        rsp: restored_rsp,
    })
}
