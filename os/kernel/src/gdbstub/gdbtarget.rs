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

const THREAD_REG_COUNT: usize = 19;

#[repr(usize)]
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
    pub registers: [usize; THREAD_REG_COUNT],
    pub rsp: u64,
}

/*impl From<ThreadContext> for X86_64CoreRegs {
    fn from(ctx: ThreadContext) -> Self {
        let mut regs = X86_64CoreRegs::default();
        regs.regs[0] = ctx.registers[ThreadRegs::Rax as usize];
        regs.regs[1] = ctx.registers[ThreadRegs::Rbx as usize];
        regs.regs[2] = ctx.registers[ThreadRegs::Rcx as usize];
        regs.regs[3] = ctx.registers[ThreadRegs::Rdx as usize];
        regs.regs[4] = ctx.registers[ThreadRegs::Rsi as usize];
        regs.regs[5] = ctx.registers[ThreadRegs::Rdi as usize];
        regs.regs[6] = ctx.registers[ThreadRegs::Rbx as usize];
        regs.regs[7] = ctx.rsp; 
        regs.regs[8] = ctx.registers.[ThreadRegs::R8 as usize];
        regs.regs[9] = ctx.registers[ThreadRegs::R9 as usize];
        regs.regs[10] = ctx.registers[ThreadRegs::R10 as usize];
        regs.regs[11] = ctx.registers[ThreadRegs::R11 as usize];
        regs.regs[12] = ctx.registers[ThreadRegs::R12 as usize];
        regs.regs[13] = ctx.registers[ThreadRegs::R13 as usize];
        regs.regs[14] = ctx.registers[ThreadRegs::R14 as usize];
        regs.regs[15] = ctx.registers.[ThreadRegs::R15 as usize];

        regs.rip = ctx.registers[ThreadRegs::Rip];
        regs.eflags = ctx.registers[ThreadRegs::Rflags];
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

    let registers = unsafe { (rsp.as_u64() as *const [usize; THREAD_REG_COUNT]).read() };

    Some(ThreadContext {
        registers,
        rsp: rsp.as_u64(),
    })
}