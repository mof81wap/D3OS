use spin::Mutex;
use crate::gdbstub::gdbtarget::{GdbSwBreakpoint, ThreadContextMut};
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
}

#[derive(Debug, Clone)]
pub struct StepOver {
    pub tid: usize,
    pub addr: u64,
    pub ctx_ptr: usize,
}

#[derive(Debug)]
pub struct DebugState {
    pub event: Option<DebugEvent>,
    pub ctrlc_pending: bool,
    pub gdbstub_is_initilaized: bool,
    pub stepping: bool,
    pub gdb_stub_tid: Option<usize>,
    pub breakpoints: Vec<GdbSwBreakpoint>,
    pub stopped_at_sw_break: Option<(usize, u64)>,
    pub stepping_over: Option<StepOver>,
    pub stopped_tid: Option<usize>,
    pub stopped_rip: Option<u64>,
}

pub static GDB_DEBUG_STATE: Mutex<DebugState> = Mutex::new(DebugState {
    event: None,
    ctrlc_pending: false,
    gdbstub_is_initilaized: false,
    stepping: false,
    gdb_stub_tid: None,
    breakpoints: Vec::new(),
    stopped_at_sw_break: None,
    stepping_over: None,
    stopped_tid: None,
    stopped_rip: None,
});