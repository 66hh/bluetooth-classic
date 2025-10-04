use std::{result, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;

use crate::common::device::BluetoothDevice;

pub mod common;

pub mod mock;

#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, thiserror::Error)]
pub enum BluetoothError {
    #[error("Permission denied")]
    PermissionDenied,

    #[error("Device not found")]
    DeviceNotFound,

    #[error("Device not pairing")]
    DeviceNotPairing,

    #[error("Service not found")]
    ServiceNotFound,

    #[error("Not connected")]
    NotConnected,

    #[error("Timed out after {:?}", _0)]
    TimedOut(Duration),

    #[error("Runtime Error: {}", _0)]
    RuntimeError(String),
}

pub type Result<T> = result::Result<T, BluetoothError>;

pub trait BluetoothSppSession: AsyncRead + AsyncWrite {
    fn connect(&mut self, device: &BluetoothDevice, need_pairing: bool) -> Result<()>;
    fn connect_timeout(
        &mut self,
        device: &BluetoothDevice,
        need_pairing: bool,
        timeout: Duration,
    ) -> Result<()>;
    fn connect_by_uuid(
        &mut self,
        device: &BluetoothDevice,
        uuid: Uuid,
        need_pairing: bool,
    ) -> Result<()>;
    fn connect_by_uuid_timeout(
        &mut self,
        device: &BluetoothDevice,
        uuid: Uuid,
        need_pairing: bool,
        timeout: Duration,
    ) -> Result<()>;
    fn connect_async(
        &mut self,
        device: &BluetoothDevice,
        need_pairing: bool,
    ) -> impl std::future::Future<Output = Result<()>>;
    fn connect_by_uuid_async(
        &mut self,
        device: &BluetoothDevice,
        uuid: Uuid,
        need_pairing: bool,
    ) -> impl std::future::Future<Output = Result<()>>;
    fn device(&self) -> &BluetoothDevice;
    fn into_device(self) -> BluetoothDevice;
}

#[cfg(test)]
mod tests {

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use crate::{
        common::mac::{mac_string_to_u64, mac_u64_to_string},
        mock::session::MockSession,
    };

    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_timeout() {
        let device = BluetoothDevice::empty();
        let mut session = MockSession::new();
        let error = session.connect_timeout(&device, true, Duration::from_secs(1));

        match error {
            Ok(_) => {}
            _ => {
                assert!(true);
            }
        }

        session.blocked_connect(true);
        let error = session.connect_timeout(&device, true, Duration::from_secs(1));

        match error {
            Err(BluetoothError::TimedOut(_)) => {}
            _ => {
                assert!(true);
            }
        }
    }

    #[test]
    fn test_read_and_write() {
        let mut session = MockSession::new();
        let data = vec![1, 2, 3];
        if let Err(_) = aw!(session.write_all(&data)) {
            assert!(true);
        }

        let mut read = [0; 3];
        if let Err(_) = aw!(session.read_exact(&mut read)) {
            assert!(true);
        }

        let read_vec: Vec<u8> = read.to_vec();

        assert_eq!(data, read_vec);
    }

    #[test]
    fn test_mac_addr_parse() {
        let addr = "00:02:B0:57:7D:D6".to_string();
        let result = mac_string_to_u64(&addr);
        if let Some(value) = result {
            if value != 11548458454 {
                assert!(true);
            }

            let text = mac_u64_to_string(value);
            assert_eq!(text, addr);
        } else {
            assert!(true);
        }
    }
}
