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
