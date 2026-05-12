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


struct GdbStubTarget;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadRegs {
    pub gsbase: u64,
    pub fsbase: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rflags: u64,
    pub rip: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadContext {
    pub registers: ThreadRegs,
    pub rsp: u64,
}

/*impl From<ThreadContext> for X86_64CoreRegs {
    fn from(ctx: ThreadContext) -> Self {
        let mut regs = X86_64CoreRegs::default();
        regs.regs[0] = ctx.registers.rax;
        regs.regs[1] = ctx.registers.rbx;
        regs.regs[2] = ctx.registers.rcx;
        regs.regs[3] = ctx.registers.rdx;
        regs.regs[4] = ctx.registers.rsi;
        regs.regs[5] = ctx.registers.rdi;
        regs.regs[6] = ctx.registers.rbp;
        regs.regs[7] = ctx.rsp; 
        regs.regs[8] = ctx.registers.r8;
        regs.regs[9] = ctx.registers.r9;
        regs.regs[10] = ctx.registers.r10;
        regs.regs[11] = ctx.registers.r11;
        regs.regs[12] = ctx.registers.r12;
        regs.regs[13] = ctx.registers.r13;
        regs.regs[14] = ctx.registers.r14;
        regs.regs[15] = ctx.registers.r15;

        regs.rip = ctx.registers.rip;
        regs.eflags = ctx.registers.rflags;
    }
}*/

/*impl Target for GdbStubTarget {
    type Error = ();
    type Arch = X86_64_SSE;

    #[inline(always)]
    fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
        BaseOps::MultiThread(self)
    }
}

impl MultiThreadBase for GdbStubTarget {
    fn read_(
        &mut self,
        regs: &mut X86_64CoreRegs,
        tid: Tid
    ) -> TargetResult<(), Self> {
        let thread = scheduler()
                    .thread(tid.get())
                    .ok_or(TargetError::NonFatal)?;

        let rsp0 = thread.saved_rsp0();
        let ctx = thread_context_from_rsp(rsp0).ok_or(TargetError::NonFatal)?;
    }

    fn write_(
        &mut self,
        regs: &X86_64CoreRegs,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        TargetError::NonFatal
    }
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
        tid: Tid
    ) -> TargetResult<usize, Self> {
        Ok(())
    }
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
        tid: Tid
    ) -> TargetResult<(), Self> {
        Ok(())
    }
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid)
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}*/

pub fn thread_context_from_rsp(rsp: VirtAddr) -> Option<ThreadContext> {
    if rsp.is_null() {
        return None;
    }

    let registers = unsafe { (rsp.as_u64() as *const ThreadRegs).read() };

    Some(ThreadContext {
        registers,
        rsp: rsp.as_u64(),
    })
}