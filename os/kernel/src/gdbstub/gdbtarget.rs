use gdbstub::target::{Target, TargetResult, TargetError};
use gdbstub_arch::x86::X86_64_SSE;
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::multithread::{MultiThreadBase, MultiThreadResume, MultiThreadSingleStep};
use gdbstub::target::ext::base::multithread::{MultiThreadResumeOps, MultiThreadSingleStepOps};
use gdbstub::common::{Tid};
use gdbstub::arch::Arch;
use crate::{scheduler};
use x86_64::VirtAddr;
use log::info;


pub struct GdbStubTarget;

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