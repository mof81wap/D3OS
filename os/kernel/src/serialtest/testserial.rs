use crate::device::serial::SerialPort;
use crate::device::serial::ComPort::{Com1, Com2};
use crate::device::serial::BaudRate::{Baud115200};
use stream::OutputStream;
use stream::DecodedInputStream;
use crate::alloc::string::ToString;
use crate::process::thread::Thread;
use crate::process::scheduler::Scheduler;
use crate::process::core_local_storage::scheduler;
use alloc::sync::Arc;
use crate::gdbstub::gdbtarget::{thread_context_from_rsp, ThreadRegs, GdbStubTarget};
use log::info;
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use gdbstub::target::ext::base::multithread::MultiThreadBase;
use gdbstub::common::Tid;
use core::ptr::addr_of_mut;
use gdbstub::target::ext::breakpoints::{Breakpoints, SwBreakpoint};
use x86_64::registers::rflags::RFlags;
use x86_64::registers::debug::{Dr0, Dr1, Dr2, Dr3, Dr7, DebugAddressRegister, Dr7Flags, Dr7Value};
use alloc::vec::Vec;
use alloc::string::String;



pub fn test_com2() {
    let mut serial = SerialPort::new_write_only(Com1);

    serial.write_byte(b'T');
    serial.write_byte(b'E');
    serial.write_byte(b'S');
    serial.write_byte(b'T');
    serial.write_byte(b'\r');
    serial.write_byte(b'\n');
}

pub fn test_serial_loop() {
    let mut serial = SerialPort::new_write_only(Com1);

    for n in 0u32..50u32 {
        serial.write_byte(b'T');
        serial.write_byte(b'E');
        serial.write_byte(b'S');
        serial.write_byte(b'T');
        serial.write_byte(b':');
        serial.write_str(&n.to_string());
        serial.write_byte(b'\r');
        serial.write_byte(b'\n');
    }
}

pub fn test_non_write_only() {
    let mut serial = SerialPort::new(Com1, Baud115200, 128);

    serial.write_byte(b'T');
    serial.write_byte(b'E');
    serial.write_byte(b'S');
    serial.write_byte(b'T');
    serial.write_byte(b'R');
    serial.write_byte(b'E');
    serial.write_byte(b'A');
    serial.write_byte(b'D');
    serial.write_byte(b'\r');
    serial.write_byte(b'\n');
}

fn hex(n: u8) -> u8 {
    match n {
        0..9 => b'0' + n,
        _ => b'a' + (n-10),
    }
}

pub extern "sysv64" fn test_read() {
    info!("ENTER TEST READ");
    let mut serial = SerialPort::new(Com2, Baud115200, 512);
    let port = Arc::new(serial);
    SerialPort::plugin(port.clone());
    port.write_str("READY\r\n");
    port.print_lsr();
    
    loop {
        if let Some(b) = port.decoded_try_read_byte() {
            port.write_byte(b as u8);
        }
    } 

    /*let port = SerialPort::new(Com1, Baud115200, 512);
    port.write_str("READY\r\n");

    loop {
        if let Some(b) = port.try_read_polled() {
            port.write_byte(b);
        }
    }*/
}

pub extern "sysv64" fn debug_thread_context_wrapper() {
    debug_thread_context();
}

pub extern "sysv64" fn debug_thread_context() -> ! {
    let tid = scheduler().current_thread().id();
    //info!("THREAD ID={}", tid);
    //info!("ENTERING DEBUG THREAD CONTEXT");
    let thread = Thread::new_kernel_thread(test_read, "a");
    //scheduler().ready(thread.clone());
    
    let rsp = thread.saved_rsp0();

    /*let ctx = match thread_context_from_rsp(rsp) {
        Some(ctx) => ctx,
        None => {
            info!("No thread context");
            return;
        }
    };*/

    loop {
        gdb_break_here();
        core::hint::spin_loop();
        scheduler().yield_now();
    }

    //info!("saved rsp={:#x}", ctx.rsp);

    /*info!("rax={:#x}", ctx.registers[ThreadRegs::Rax as usize]);
    info!("rbx={:#x}", ctx.registers[ThreadRegs::Rbx as usize]);
    info!("rcx={:#x}", ctx.registers[ThreadRegs::Rcx as usize]);
    info!("rdx={:#x}", ctx.registers[ThreadRegs::Rdx as usize]);
    info!("rsi={:#x}", ctx.registers[ThreadRegs::Rsi as usize]);
    info!("rdi={:#x}", ctx.registers[ThreadRegs::Rdi as usize]);
    info!("rbp={:#x}", ctx.registers[ThreadRegs::Rbp as usize]);
    info!("r8={:#x}", ctx.registers[ThreadRegs::R8 as usize]);
    info!("r9={:#x}", ctx.registers[ThreadRegs::R9 as usize]);
    info!("r10={:#x}", ctx.registers[ThreadRegs::R10 as usize]);
    info!("r11={:#x}", ctx.registers[ThreadRegs::R11 as usize]);
    info!("r12={:#x}", ctx.registers[ThreadRegs::R12 as usize]);
    info!("r13={:#x}", ctx.registers[ThreadRegs::R13 as usize]);
    info!("r14={:#x}", ctx.registers[ThreadRegs::R14 as usize]);
    info!("r15={:#x}", ctx.registers[ThreadRegs::R15 as usize]);
    info!("rflags={:#x}", ctx.registers[ThreadRegs::Rflags as usize]);*/

    test_gdb_target_ops();
    test_gdb_write_registers();
    test_gdb_write_addrs();
    //test_sw_breakpoint();
}

pub fn test_trap_flag_once() {
    info!("before manual TF test");

    unsafe {
        x86_64::registers::rflags::write(
            x86_64::registers::rflags::read() | RFlags::TRAP_FLAG
        );

        core::arch::asm!("nop", options(nomem, nostack, preserves_flags));
    }

    info!("after manual TF test");
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn gdb_break_here() {
    let x = 1;
    let y = 2;
    hex(32);
    //info!("GDB BREAK HERE DR7={:?}", Dr7::read());
    scheduler();
    let z = x + y;
    let dr7 = Dr7::read();
    //info!("GDB BREAK HERE");
    let mut list = Vec::new();
    list.push(1);
    list.push(42);
    list.push(123432);
    list.push(-1);
    let test_struct = GdbTestStruct::new();

    for i in 0..1000000 {
        let mut x = 0;
        x = i + 1;
    }
}

pub fn test_gdb_target_ops() {
    for n in 1..=4 {
        let mut target = GdbStubTarget::new();
        let mut regs = X86_64CoreRegs::default();
        let tid = Tid::new(n).unwrap();

        match <GdbStubTarget as MultiThreadBase>::read_registers(&mut target, &mut regs, tid) {
            Ok(()) => {
                info!("read_registers ok");
                info!("rip={:#x} rsp={:#x} rax={:#x}", regs.rip, regs.regs[7], regs.regs[0]);
                info!("rbx={:#x} rcx={:#x} rdx={:#x}", regs.regs[1], regs.regs[2], regs.regs[3]);
            }
            Err(e) => {
                info!("read_registers failed");
            }
        }

        let mut buf = [0u8; 16];
        let rip = regs.rip;

        match <GdbStubTarget as MultiThreadBase>::read_addrs(&mut target, rip, &mut buf, tid) {
            Ok(n) => {
                info!("read_addrs ok: {} bytes at rip={:#x}", n, rip);
                for b in buf.iter() {
                    info!("byte={:#x}", *b);
                }
            }
            Err(e) => {
                info!("read_addrs failed");
            }
        }

        let mut count: usize = 0;
        let result = <GdbStubTarget as MultiThreadBase>::list_active_threads(
            &mut target,
            &mut |tid: Tid| {
                count += 1;
                info!("active tid={}", tid.get());
            }
        );
        info!("list_active_threads result={:?}, count={}", result, count);
    }
}

pub fn test_gdb_write_registers() {
    info!("ENTERING WRITE REGS");
    let mut target = GdbStubTarget::new();
    let tid = Tid::new(1).unwrap();

    let mut before = X86_64CoreRegs::default();
    if <GdbStubTarget as MultiThreadBase>::read_registers(&mut target, &mut before, tid).is_err() {
        info!("read_registers failed");
    }

    info!("before rax={:#x}", before.regs[0]);

    let mut modified = before.clone();
    modified.regs[0] = 0x1234_5678_9abc_def0;

    if <GdbStubTarget as MultiThreadBase>::write_registers(&mut target, &modified, tid).is_err() {
        info!("write_registers failed");
    }

    let mut after = X86_64CoreRegs::default();
    if <GdbStubTarget as MultiThreadBase>::read_registers(&mut target, &mut after, tid).is_err() {
        info!("read_registers failed");
    }

    info!("after rax={:#x}", after.regs[0]);

    if <GdbStubTarget as MultiThreadBase>::write_registers(&mut target, &before, tid).is_err() {
        info!("write_registers failed");
    }
}

static mut GDB_TEST_MEM: [u8; 8] = [0xaa; 8];

pub fn test_gdb_write_addrs() {
    let mut target = GdbStubTarget::new();
    let tid = Tid::new(1).unwrap();

    let addr = unsafe { addr_of_mut!(GDB_TEST_MEM) as *mut u8 as u64 };

    let mut before = [0u8; 8];
    if <GdbStubTarget as MultiThreadBase>::read_addrs(&mut target, addr, &mut before, tid).is_err() {
        info!("read_addrs failed");
    }
    info!("before={:?}", before);

    let data = [1, 2, 3, 4, 5, 6, 7, 8];
    if <GdbStubTarget as MultiThreadBase>::write_addrs(&mut target, addr, &data, tid).is_err() {
        info!("write_addrs failed");
    }

    let mut after = [0u8; 8];
    if <GdbStubTarget as MultiThreadBase>::read_addrs(&mut target, addr, &mut after, tid).is_err() {
        info!("read_addrs failed");
    }
    info!("after={:?}", after);

    if <GdbStubTarget as MultiThreadBase>::write_addrs(&mut target, addr, &before, tid).is_err() {
        info!("write_addrs failed");
    }
}

pub extern "sysv64" fn breakpoint_test_target() {
    info!("before breakpoint target");
    info!("inside breakpoint target");
    info!("after breakpoint target");
}

pub fn test_sw_breakpoint() {
    let mut target = GdbStubTarget::new();
    let addr = breakpoint_test_target as usize as  u64;
    info!("bp target addr={:#x}", addr);

    let ok = <GdbStubTarget as SwBreakpoint>::add_sw_breakpoint(&mut target, addr, 1);
    info!("add_sw_breakpoint ok={}", matches!(ok, Ok(true)));

    breakpoint_test_target();

    let ok = <GdbStubTarget as SwBreakpoint>::remove_sw_breakpoint(&mut target, addr, 1);
    info!("returned from breakpoint target");
}

struct GdbTestStruct {
    pub int_field: usize,
    pub string_field: String,
    pub array_field: [usize; 5],
}

impl GdbTestStruct {
    pub fn new() -> Self {
        Self {
            int_field: 42,
            string_field: "Hello World".to_string(),
            array_field: [1, 42, 8775, 2432, 2],
        }
    }
}