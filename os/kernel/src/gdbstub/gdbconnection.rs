use crate::device::serial::SerialPort;
use crate::device::serial::ComPort::{Com1, Com2};
use crate::device::serial::BaudRate::{Baud115200};
use stream::OutputStream;
use stream::DecodedInputStream;
use gdbstub::conn::Connection;

struct GdbStubConnection {
    serial_port: SerialPort,
}

impl GdbStubConnection {
    fn new() -> Self {
        let serial_port = SerialPort::new(Com1, Baud115200, 128);
        Self { 
            serial_port
        }
    }

    fn read(&self) -> Option<u8> {
        self.serial_port.try_read_polled()
    }
}

impl Connection for GdbStubConnection {
    type Error = ();

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.serial_port.write_byte(byte);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}