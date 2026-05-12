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
use crate::gdbstub::gdbtarget::{thread_context_from_rsp};
use log::info;


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

    info!("rax={:#x}", ctx.registers.rax);
    info!("rbx={:#x}", ctx.registers.rbx);
    info!("rcx={:#x}", ctx.registers.rcx);
    info!("rdx={:#x}", ctx.registers.rdx);
    info!("rsi={:#x}", ctx.registers.rsi);
    info!("rdi={:#x}", ctx.registers.rdi);
    info!("rbp={:#x}", ctx.registers.rbp);
    info!("r8={:#x}", ctx.registers.r8);
    info!("r9={:#x}", ctx.registers.r9);
    info!("r10={:#x}", ctx.registers.r10);
    info!("r11={:#x}", ctx.registers.r11);
    info!("r12={:#x}", ctx.registers.r12);
    info!("r13={:#x}", ctx.registers.r13);
    info!("r14={:#x}", ctx.registers.r14);
    info!("r15={:#x}", ctx.registers.r15);
    info!("rflags={:#x}", ctx.registers.rflags);
}