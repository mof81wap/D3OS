pub enum DebugEvent {
    SwBreakpoint {
        tid: usize,
        addr: u64,
    },
}

#[derive(Debug)]
pub struct DebugState {
    pub event: Option<DebugEvent>,
}

pub static GDB_DEBUG_STATE: Mutex<DebugState> = Mutex::new(DebugState {
    event: None,
})