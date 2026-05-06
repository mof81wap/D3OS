use gdbstub::target::{Target, TargetResult};
use gdbstub_arch::x86::X86_64_SSE;
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::base::singlethread::{SingleThreadBase, SingleThreadResume, SingleThreadSingleStep};
use gdbstub::target::ext::base::singlethread::{SingleThreadResumeOps, SingleThreadSingleStepOps};
use crate::{scheduler};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ThreadId(usize);

struct GdbStubTarget {
    selected_thread: ThreadId,
    stopped_thread: ThreadId,
}

impl Target for GdbStubTarget {
    type Error = ();
    type Arch = X86_64_SSE;

    #[inline(always)]
    fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
        BaseOps::SingleThread(self)
    }
}

impl SingleThreadBase for GdbStubTarget {
    fn read_registers(&mut self, regs: &mut X86_64CoreRegs) -> TargetResult<(), Self> { todo!() }

    fn write_registers(&mut self, regs: &X86_64CoreRegs) -> TargetResult<(), Self> { todo!() }

    fn read_addrs(&mut self, start_addr: u64, data: &mut [u8]) -> TargetResult<(), Self> { todo!() }

    fn write_addrs(&mut self, start_addr: u64, data: &[u8]) -> TargetResult<(), Self> { todo!() }
}

impl GdbStubTarget {
    fn new() -> Self {
        let (_current_pid, current_tid) = scheduler().current_ids();

        Self {
            selected_thread: current_tid,
            stopped_thread: current_tid,
        }
    }
}