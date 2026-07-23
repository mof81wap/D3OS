use spin::Mutex;
use crate::gdbstub::gdbtarget::{GdbSwBreakpoint, ThreadContextMut, GdbHwBreakpoint};
use alloc::vec::Vec;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::VirtAddr;

#[derive(Debug, PartialEq, Clone)]
pub enum DebugEvent {
    SwBreakpoint {
        tid: usize,
        addr: u64,
    },
    CtrlC,
    SingleStep {
        tid: usize,
    },
    HwBreakpoint {
        tid: usize,
    }
}

#[derive(Debug)]
pub struct DebugState {
    pub event: Mutex<Option<DebugEvent>>,
    pub gdb_stub_tid: Mutex<Option<usize>>,
    pub breakpoints: Mutex<Vec<GdbSwBreakpoint>>,
    pub hwbreakpoints: Mutex<[GdbHwBreakpoint; 4]>,
}

#[cfg(feature = "gdbstub")]
pub static GDB_DEBUG_STATE: DebugState = DebugState {
    event: Mutex::new(None),
    gdb_stub_tid: Mutex::new(None),
    breakpoints: Mutex::new(Vec::new()),
    hwbreakpoints: Mutex::new([GdbHwBreakpoint{address: 0}; 4]),
};