use gdbstub::target::{Target, TargetResult, TargetError};
use gdbstub_arch::x86::X86_64_SSE;
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::multithread::{MultiThreadBase, MultiThreadResume, MultiThreadSingleStep, MultiThreadSchedulerLocking};
use gdbstub::target::ext::base::multithread::{MultiThreadResumeOps, MultiThreadSingleStepOps, MultiThreadSchedulerLockingOps};
use gdbstub::common::{Tid};
use gdbstub::arch::Arch;
use crate::{scheduler, process_manager};
use x86_64::VirtAddr;
use gdbstub::target::ext::breakpoints::{Breakpoints, SwBreakpoint, HwBreakpoint};
use gdbstub::target::ext::breakpoints::{BreakpointsOps, SwBreakpointOps, HwBreakpointOps};
use crate::process::process::Process;
use alloc::sync::Arc;
use log::info;
use volatile::Volatile;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::registers::rflags::RFlags;
use spin::Mutex;
use alloc::vec::Vec;
use crate::gdbstub::debug_state::{GDB_DEBUG_STATE, DebugEvent, StepOver};
use gdbstub::stub::MultiThreadStopReason;
use crate::device::cpu::{disable_int_nested, enable_int_nested};
use crate::process::thread::ThreadState;
use gdbstub::common::Signal;
use x86_64::registers::control::{Cr0, Cr3};
use x86_64::structures::paging::page::{PageRange, Size4KiB, Page};
use x86_64::structures::paging::PageTableFlags;
use x86_64::registers::debug::{Dr0, Dr1, Dr2, Dr3, Dr6, Dr7, DebugAddressRegister, Dr7Flags, Dr7Value, Dr6Flags};


pub struct GdbStubTarget {
    selected_pid: Arc<Process>,
    resume_actions: Mutex<Vec<(usize, ResumeAction)>>,
}

impl GdbStubTarget {
    pub fn new() -> Self {
        let selected_pid = process_manager().read().kernel_process().unwrap();

        Self {
            selected_pid,
            resume_actions: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GdbSwBreakpoint {
    address: u64,
    instruction: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct GdbHwBreakpoint {
    pub address: u64,
}

#[derive(Clone, Copy, Debug)]
enum ResumeAction {
    Continue,
    SingleStep,
}

const THREAD_REG_COUNT: usize = 19;
const RFLAGS_TF: u64 = 0x100;

#[repr(u64)]
#[derive(Clone, Copy, Debug)]
pub enum ThreadRegs {
    Gsbase, 
    Fsbase, 
    Rbp, 
    Rdi, 
    Rsi, 
    Rdx, 
    Rcx, 
    Rbx, 
    Rax, 
    R15,
    R14, 
    R13, 
    R12, 
    R11, 
    R10, 
    R9, 
    R8,
    Rflags,
    Rip,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadContext {
    pub registers: [u64; THREAD_REG_COUNT],
    pub rsp: u64,
}

#[repr(C)]
#[derive(Debug)]
pub struct ThreadContextMut<'a> {
    pub registers: &'a mut [u64; THREAD_REG_COUNT],
    pub rsp: u64,
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

#[repr(C)]
#[derive(Debug)]
pub struct GdbTrapFrame {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

fn regs_from_trap_frame(frame: &GdbTrapFrame) -> X86_64CoreRegs {
    let mut regs = X86_64CoreRegs::default();

    regs.regs[0] = frame.rax;
    regs.regs[1] = frame.rbx;
    regs.regs[2] = frame.rcx;
    regs.regs[3] = frame.rdx;
    regs.regs[4] = frame.rsi;
    regs.regs[5] = frame.rdi;
    regs.regs[6] = frame.rbp;
    regs.regs[7] = frame.rsp;

    regs.regs[8] = frame.r8;
    regs.regs[9] = frame.r9;
    regs.regs[10] = frame.r10;
    regs.regs[11] = frame.r11;
    regs.regs[12] = frame.r12;
    regs.regs[13] = frame.r13;
    regs.regs[14] = frame.r14;
    regs.regs[15] = frame.r15;

    regs.rip = frame.rip;
    regs.eflags = frame.rflags as u32;

    regs
}

unsafe extern "C" {
    pub fn gdb_breakpoint_entry();
    pub fn gdb_debug_entry();
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

        if let Some(ptr) = thread.debug_trap_frame() {
            let frame = unsafe { ptr.as_ref() };
            *regs = regs_from_trap_frame(frame);

            return Ok(());
        }

        let rsp0 = thread.saved_rsp0();
        let ctx = thread_context_from_rsp(rsp0).ok_or(TargetError::NonFatal)?;
        *regs = X86_64CoreRegs::from(ctx);

        let stopped_rip = {
            let state = GDB_DEBUG_STATE.lock();
            if state.stopped_tid == Some(tid.get()) {
                state.stopped_rip
            } else {
                None
            }
        };

        if let Some(rip) = stopped_rip {
            regs.rip = rip;
        }

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

        if let Some(ptr) = thread.debug_trap_frame() {
            let frame = unsafe { &mut *ptr.as_ptr() };

            frame.rax = regs.regs[0];
            frame.rbx = regs.regs[1];
            frame.rcx = regs.regs[2];
            frame.rdx = regs.regs[3];
            frame.rsi = regs.regs[4];
            frame.rdi = regs.regs[5];
            frame.rbp = regs.regs[6];
            frame.rsp = regs.regs[7];
            frame.r8 = regs.regs[8];
            frame.r9 = regs.regs[9];
            frame.r10 = regs.regs[10];
            frame.r11 = regs.regs[11];
            frame.r12 = regs.regs[12];
            frame.r13 = regs.regs[13];
            frame.r14 = regs.regs[14];
            frame.r15 = regs.regs[15];

            frame.rip = regs.rip;
            frame.rflags = regs.eflags as u64;

            return Ok(());
        } 
        
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

            unsafe { *byte = (virt_addr as *const u8).read(); }
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

    #[inline(always)]
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid)
    ) -> Result<(), Self::Error> {
        let active_ids = scheduler().gdb_thread_ids();

        for id in active_ids {
            if id != 0 {
                thread_is_active(Tid::new(id).unwrap());
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn support_resume(&mut self) -> Option<MultiThreadResumeOps<Self>> {
        Some(self)
    }
}

impl Breakpoints for GdbStubTarget {
    #[inline(always)]
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_hw_breakpoint(&mut self) -> Option<HwBreakpointOps<Self>> {
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

        {
            let state = GDB_DEBUG_STATE.lock();
            if state.breakpoints.iter().any(|bp| bp.address == virt_addr) {
                return Ok(true);
            }
        }

        let instruction = unsafe { virt_addr_ptr.read_volatile() };
        unsafe { virt_addr_ptr.write_volatile(0xCC) };

        {
            let mut state = GDB_DEBUG_STATE.lock();
            state.breakpoints.push(GdbSwBreakpoint{address: virt_addr, instruction});
        }

        Ok(true)
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        let virt_addr = addr as u64;
        let virt_addr_ptr = unsafe { virt_addr as *mut u8 };

        let bp = {
            let mut state = GDB_DEBUG_STATE.lock();
            let Some(index) = state.breakpoints.iter().position(|bp| bp.address == virt_addr) else {
                return Ok(false);
            };
            state.breakpoints.remove(index)
        };

        let instruction = bp.instruction;
        unsafe { virt_addr_ptr.write_volatile(instruction) };

        Ok(true)
    }
}

impl MultiThreadResume for GdbStubTarget {

    fn resume(&mut self) -> Result<(), Self::Error> {
        let actions = self.resume_actions.lock()
                                        .clone();

        let sw_break = {
            GDB_DEBUG_STATE.lock().stopped_at_sw_break
        };

        /*if sw_break.is_none() {
            scheduler().debug_resume_all();
            enable_int_nested(true);
            scheduler().yield_now();
            return Ok(());
        }*/

        if actions.is_empty() {

            let stopped = {
                let mut state = GDB_DEBUG_STATE.lock();
                state.stepping = false;
                state.event = None;
                state.stopped_rip = None;
                state.stopped_tid = None;
                state.stopped_at_sw_break.take()
            };

            if let Some((tid, addr)) = stopped {
                scheduler().debug_resume_thread(tid);
                scheduler().debug_resume_all();
            } else {
                scheduler().debug_resume_all();

                for i in 1..10 {
                    scheduler().debug_resume_thread(i);
                }
            }

            enable_int_nested(true);
            return Ok(());
        }

        for (tid, action) in actions.iter() {
            match action {
                ResumeAction::Continue => {
                    {
                        let mut state = GDB_DEBUG_STATE.lock();
                        state.stepping = false;
                        state.event = None;
                        state.stopped_at_sw_break = None;
                        state.stopped_tid = None;
                        state.stopped_rip = None;
                    }
                    scheduler().debug_resume_thread(*tid);
                }
                ResumeAction::SingleStep => {
                    let thread = scheduler()
                            .thread(*tid)
                            .ok_or(())?;
                    {
                        let mut state = GDB_DEBUG_STATE.lock();
                        state.stepping = true;
                        state.event = None;
                    }

                    if let Some(ptr) = thread.debug_trap_frame() {
                        let frame = unsafe { &mut *ptr.as_ptr() };
                        frame.rflags |= RFLAGS_TF;
                    } else {
                        let rsp = thread.saved_rsp0();
                        let mut ctx = mut_thread_context_from_rsp(rsp).ok_or(())?;
                        ctx.registers[ThreadRegs::Rflags as usize] |= RFLAGS_TF;
                    }

                    scheduler().debug_resume_thread(*tid);
                    return Ok(());
                }
            }
        }

        {
            let mut state = GDB_DEBUG_STATE.lock();
            state.event = None;
        }
        enable_int_nested(true);
        Ok(())
    }

    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        self.resume_actions
            .lock()
            .clear(); 
        Ok(())
    }

    fn set_resume_action_continue(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        self.resume_actions
            .lock()
            .push((tid.get(), ResumeAction::Continue));
        Ok(())
    }

    #[inline(always)]
    fn support_single_step(&mut self) -> Option<MultiThreadSingleStepOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_scheduler_locking(
    &mut self,
    ) -> Option<MultiThreadSchedulerLockingOps<'_, Self>> {
        Some(self)
    }
}

impl MultiThreadSingleStep for GdbStubTarget {
    fn set_resume_action_step(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        self.resume_actions
            .lock()
            .push((tid.get(), ResumeAction::SingleStep));

            Ok(())
    }
}

impl MultiThreadSchedulerLocking for GdbStubTarget {
    fn set_resume_action_scheduler_lock(
        &mut self
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl HwBreakpoint for GdbStubTarget {
    fn add_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        let virt_addr = addr as u64;

        {
            let mut state = GDB_DEBUG_STATE.lock();
            if state.hwbreakpoints.iter().any(|bp| bp.address == virt_addr) {
                return Ok(true);
            }

            let Some(index) = state.hwbreakpoints.iter().position(|bp| bp.address == 0)
            else {
                return Ok(false);
            };

            state.hwbreakpoints[index].address = virt_addr;
            let mut dr7 = Dr7::read();
            info!("DR7={:?}", dr7);

            match index {
                0 => {
                    Dr0::write(virt_addr);
                    dr7.insert_flags(Dr7Flags::GLOBAL_BREAKPOINT_0_ENABLE);
                    Dr7::write(dr7);
                },
                1 => {
                    Dr1::write(virt_addr);
                    dr7.insert_flags(Dr7Flags::GLOBAL_BREAKPOINT_1_ENABLE);
                    Dr7::write(dr7);
                },
                2 => {
                    Dr2::write(virt_addr);
                    dr7.insert_flags(Dr7Flags::GLOBAL_BREAKPOINT_2_ENABLE);
                    Dr7::write(dr7);
                },
                3 => {
                    Dr3::write(virt_addr);
                    dr7.insert_flags(Dr7Flags::GLOBAL_BREAKPOINT_3_ENABLE);
                    Dr7::write(dr7);
                },
                _ => {},
            }
        }
        info!("DR7={:?}", Dr7::read());
        Ok(true)
    }

    fn remove_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        let virt_addr = addr as u64;

        {
            let mut state = GDB_DEBUG_STATE.lock();
            if let Some(index) = state.hwbreakpoints.iter().position(|bp| bp.address == virt_addr) {
                state.hwbreakpoints[index].address = 0;
                let mut dr7 = Dr7::read();
                match index {
                    0 => {
                        Dr0::write(virt_addr);
                        dr7.remove_flags(Dr7Flags::GLOBAL_BREAKPOINT_0_ENABLE);
                        Dr7::write(dr7);
                    },
                    1 => {
                        Dr1::write(virt_addr);
                        dr7.remove_flags(Dr7Flags::GLOBAL_BREAKPOINT_1_ENABLE);
                        Dr7::write(dr7);
                    },
                    2 => {
                        Dr2::write(virt_addr);
                        dr7.remove_flags(Dr7Flags::GLOBAL_BREAKPOINT_2_ENABLE);
                        Dr7::write(dr7);
                    },
                    3 => {
                        Dr3::write(virt_addr);
                        dr7.remove_flags(Dr7Flags::GLOBAL_BREAKPOINT_3_ENABLE);
                        Dr7::write(dr7);
                    },
                    _ => {},
                }
                return Ok(true);
            } else {
                return Ok(false);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gdb_interrupt_handler(frame: *mut GdbTrapFrame, index: u64) {
    let frame = unsafe { &mut *frame };

    match index {
        1 => gdb_handle_debug_exception(frame),
        3 => gdb_handle_int3(frame),
        _ => panic!("unexpected gdb trap index {}", index),
    }
}

fn gdb_handle_int3(frame: &mut GdbTrapFrame) {
    disable_int_nested();

    let bp_addr = frame.rip - 1;
    frame.rip = bp_addr;
    frame.rflags &= !RFLAGS_TF;

    let thread = scheduler()
        .try_get_current_thread()
        .expect("FAILED TO GET CURRENT THREAD");
    
    let tid = thread.id();
    thread.set_debug_trap_frame(frame as *mut GdbTrapFrame as u64);

    let gdb_stub_tid = {
        let mut state = GDB_DEBUG_STATE.lock();
        state.stopped_at_sw_break = Some((tid, bp_addr));
        state.stopped_tid = Some(tid);
        state.stopped_rip = Some(bp_addr);

        state.event = Some(DebugEvent::SwBreakpoint {
            tid,
            addr: bp_addr,
        });
        state.gdb_stub_tid.unwrap()
    };

    scheduler().debug_stop_all_except(gdb_stub_tid);
    scheduler().debug_resume_thread(gdb_stub_tid);
    scheduler().switch_thread_from_interrupt();
}

fn gdb_handle_debug_exception(frame: &mut GdbTrapFrame) {
    disable_int_nested();
    let rip = frame.rip;
    let dr6 = Dr6::read();
    let hit_hw_bp = dr6.intersects(
        Dr6Flags::TRAP0 |
        Dr6Flags::TRAP1 |
        Dr6Flags::TRAP2 |
        Dr6Flags::TRAP3
    );
    
    let stepping = {
        GDB_DEBUG_STATE.lock().stepping
    };

    frame.rflags &= !RFLAGS_TF;

    if !stepping && !hit_hw_bp {
        return;
    }

    let thread = scheduler()
        .try_get_current_thread()
        .expect("FAILED TO GET CURRENT THREAD");

    let tid = thread.id();
    thread.set_debug_trap_frame(frame as *mut GdbTrapFrame as u64);

    let gdb_stub_tid = {
        let mut state = GDB_DEBUG_STATE.lock();

        state.stepping = false;
        state.stepping_over = None;
        //state.stopped_at_sw_break = None;
        state.stopped_tid = Some(tid);
        state.stopped_rip = Some(rip);
        if hit_hw_bp {
            state.event = Some(DebugEvent::HwBreakpoint{ tid });
        } else {
            state.event = Some(DebugEvent::SingleStep { tid });
        }
        state.gdb_stub_tid.unwrap()
    };

    {
        scheduler().debug_stop_all_except(gdb_stub_tid);
    }

    scheduler().debug_resume_thread(gdb_stub_tid);
    scheduler().switch_thread_from_interrupt();
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
