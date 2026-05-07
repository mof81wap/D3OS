use gdbstub::target::{Target, TargetResult};
use gdbstub_arch::x86::X86_64_SSE;
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::multithread::{MultiThreadBase, MultiThreadResume, MultiThreadSingleStep};
use gdbstub::target::ext::base::multithread::{MultiThreadResumeOps, MultiThreadSingleStepOps};
use crate::{scheduler};


struct GdbStubTarget;

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
        regs: &mut <Self::Arch as Arch>::Registers,
        tid: Tid
    ) -> TargetResult<(), Self>;
    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
        tid: Tid,
    ) -> TargetResult<(), Self>;
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
        tid: Tid
    ) -> TargetResult<usize, Self>;
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
        tid: Tid
    ) -> TargetResult<(), Self>;
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid)
    ) -> Result<(), Self::Error>;
}
