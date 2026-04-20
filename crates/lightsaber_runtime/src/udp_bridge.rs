use std::net::UdpSocket;
use std::time::Duration;

use lightsaber_core::ActionCommand;

use crate::protocol::parse_action_command_json;

pub struct UdpGestureReceiver {
    socket: UdpSocket,
}

impl UdpGestureReceiver {
    pub fn bind(address: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(address)?;
        socket.set_nonblocking(true)?;
        socket.set_read_timeout(Some(Duration::from_millis(5)))?;
        Ok(Self { socket })
    }

    pub fn drain(&self) -> Vec<ActionCommand> {
        let mut commands = Vec::new();

        loop {
            let mut buffer = [0_u8; 2048];
            match self.socket.recv_from(&mut buffer) {
                Ok((size, _addr)) => {
                    if let Ok(payload) = std::str::from_utf8(&buffer[..size]) {
                        if let Some(command) = parse_action_command_json(payload) {
                            commands.push(command);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_error) => break,
            }
        }

        commands
    }
}
