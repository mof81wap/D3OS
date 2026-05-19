use crate::device::serial::SerialPort;
use crate::device::serial::ComPort::{Com1, Com2};
use crate::device::serial::BaudRate::{Baud115200};
use stream::OutputStream;
use stream::DecodedInputStream;
use crate::alloc::string::ToString;
use crate::process::thread::Thread;
use crate::process::scheduler::Scheduler;
use crate::scheduler;
use alloc::sync::Arc;
use crate::gdbstub::gdbtarget::{thread_context_from_rsp, ThreadRegs, GdbStubTarget};
use log::info;
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use gdbstub::target::ext::base::multithread::MultiThreadBase;
use gdbstub::common::Tid;


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

pub extern "sysv64" fn debug_thread_context() {
    let thread = Thread::new_kernel_thread(test_read, "a");
    
    let rsp = thread.saved_rsp0();

    let ctx = match thread_context_from_rsp(rsp) {
        Some(ctx) => ctx,
        None => {
            info!("No thread context");
            return;
        }
    };

    info!("saved rsp={:#x}", ctx.rsp);

    info!("rax={:#x}", ctx.registers[ThreadRegs::Rax as usize]);
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
    info!("rflags={:#x}", ctx.registers[ThreadRegs::Rflags as usize]);

    test_gdb_target_ops();
}

pub fn test_gdb_target_ops() {
    for n in 1..=4 {
        let mut target = GdbStubTarget;
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