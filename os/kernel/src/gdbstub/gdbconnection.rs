use crate::device::serial::SerialPort;
use crate::device::serial::ComPort::{Com1, Com2, Com3};
use crate::device::serial::BaudRate::{Baud115200};
use stream::OutputStream;
use stream::DecodedInputStream;
use gdbstub::conn::{Connection, ConnectionExt};
use log::info;

pub struct GdbStubConnection {
    serial_port: SerialPort,
    peeked: Option<u8>,
}

impl GdbStubConnection {
    pub fn new() -> Self {
        let serial_port = SerialPort::new(Com3, Baud115200, 128);
        Self { 
            serial_port,
            peeked: None,
        }
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

impl ConnectionExt for GdbStubConnection {
    fn read(&mut self) -> Result<u8, Self::Error> {
        if let Some(byte) = self.peeked.take() {
            info!("READ: {:#x}", byte);
            return Ok(byte);
        }

        loop {
            if let Some(byte) = self.serial_port.try_read_polled() {
                info!("READ: {:#x}", byte);
                return Ok(byte);
            }
        }
    }

    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        if let Some(byte) = self.peeked {
            info!("PEEKED: {:#x}", byte);
            return Ok(Some(byte));
        }

        if let Some(byte) = self.serial_port.try_read_polled() {
            self.peeked = Some(byte);
            info!("PEEKED: {:#x}", byte);
            return Ok(Some(byte));
        }

        Ok(None)
    }
}