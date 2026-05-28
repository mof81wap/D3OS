use spin::Mutex;

#[derive(Debug)]
pub enum DebugEvent {
    SwBreakpoint {
        tid: usize,
        addr: u64,
    },
    CtrlC,
}

#[derive(Debug)]
pub struct DebugState {
    pub event: Option<DebugEvent>,
    pub ctrlc_pending: bool,
}

pub static GDB_DEBUG_STATE: Mutex<DebugState> = Mutex::new(DebugState {
    event: None,
    ctrlc_pending: false,
});