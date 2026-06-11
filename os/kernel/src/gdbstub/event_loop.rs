use gdbstub::common::Signal;
use gdbstub::stub::GdbStub;
use gdbstub::stub::MultiThreadStopReason;
use gdbstub::target::Target;
use gdbstub::stub::run_blocking::{BlockingEventLoop, Event, WaitForStopReasonError};
use crate::gdbstub::debug_state::{GDB_DEBUG_STATE, DebugEvent};
use core::arch::asm;
use crate::gdbstub::gdbtarget::GdbStubTarget;
use crate::gdbstub::gdbconnection::GdbStubConnection;
use gdbstub::conn::{Connection, ConnectionExt};
use gdbstub::common::Tid;
use log::info;
use gdbstub::target::ext::breakpoints::SwBreakpoint;
use crate::device::cpu::{disable_int_nested};
use crate::{scheduler};

enum GdbBlockingEventLoop{}

pub extern "sysv64" fn init_gdb_stub() {
    let tid = scheduler().current_thread().id();
    info!("GDB INIT TID={}", tid);

    {
        let mut state = GDB_DEBUG_STATE.lock();
        state.gdbstub_is_initilaized = true;
        state.gdb_stub_tid = Some(tid);
    }

    let conn = GdbStubConnection::new();
    let mut target = GdbStubTarget::new();
    disable_int_nested();

    match GdbStub::builder(conn)
        .with_packet_buffer(&mut [0u8; 4096])
        .build()
        .expect("ERROR init_gdb_stub")
        .run_blocking::<GdbBlockingEventLoop>(&mut target)
    {
        Ok(_) => {}
        Err(e) => {
            panic!("GDB stub exited: {:?}", e);
        }
    }
    panic!("GDB stub thread must never exit");
}

impl BlockingEventLoop for GdbBlockingEventLoop {
    type Target = GdbStubTarget;
    type Connection = GdbStubConnection;
    type StopReason = MultiThreadStopReason<u64>;

    fn wait_for_stop_reason(
        target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> Result<Event<Self::StopReason>, WaitForStopReasonError<<Self::Target as Target>::Error, <Self::Connection as Connection>::Error>> {
        loop {
            if conn
            .peek()
            .map_err(WaitForStopReasonError::Connection)?
            .is_some() {
                let byte = conn
                .read()
                .map_err(WaitForStopReasonError::Connection)?;
                return Ok(Event::IncomingData(byte));
            }

            let event = {
                let mut state = GDB_DEBUG_STATE.lock();
                state.event.take()
            };

            if let Some(event) = event {
                match event {
                    DebugEvent::SwBreakpoint { tid, addr: _ } => {
                        let tid = Tid::new(tid).unwrap();

                        return Ok(Event::TargetStopped(MultiThreadStopReason::SwBreak(tid)));
                    }
                    DebugEvent::CtrlC => {
                        return Ok(Event::TargetStopped(MultiThreadStopReason::Signal(Signal::SIGINT)));
                    }
                }
            }
            scheduler().yield_now();
        }
    }

    fn on_interrupt(
        target: &mut Self::Target,
    ) -> Result<Option<Self::StopReason>, <Self::Target as Target>::Error> {
        {
            let mut state = GDB_DEBUG_STATE.lock();
            state.ctrlc_pending = true;
        }

        disable_int_nested();

        Ok(None)
    }
}
