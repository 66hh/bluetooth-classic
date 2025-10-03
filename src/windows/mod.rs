pub mod uuid;
pub mod utils;
pub mod session;

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use crate::{common::device::{BluetoothDevice, SPP_UUID}, windows::{session::WinrtSession, uuid::create_service_id}, BluetoothSppSession};

    #[test]
    fn test_service_id() {

        let service_id;
        match create_service_id(SPP_UUID) {
            Ok(id) => service_id = id,
            Err(_) => {
                assert!(true);
                return;
            },
        }

        let id_str;
        match service_id.AsString() {
            Ok(str) => id_str = str.to_string(),
            Err(_) => {
                assert!(true);
                return;
            },
        }

        assert_eq!(id_str, "{00001101-0000-1000-8000-00805F9B34FB}")
    }

    #[test]
    fn test_connect() {
        let mut winrt = WinrtSession::new();
        let device = BluetoothDevice::new_by_addr_string("Test".to_string(), &"D0:AE:05:05:1A:22".to_string()).unwrap();

        let err = winrt.connect_timeout(&device, true, Duration::from_secs(500));
        if let Err(e) = err {
            println!("{}", e.to_string())
        }
    }

}