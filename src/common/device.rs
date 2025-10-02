use uuid::{uuid, Uuid};

use crate::common::mac::{mac_string_to_u64, mac_u64_to_string};

pub static SPP_UUID: Uuid = uuid!("00001101-0000-1000-8000-00805F9B34FB");

#[derive(Clone)]
pub struct BluetoothDevice {
    pub name: String,
    pub addr: u64,
}

impl BluetoothDevice {
    
    pub fn new(name: String, addr: u64) -> BluetoothDevice {
        return BluetoothDevice {
            name: name.clone(),
            addr: addr
        };
    }

    pub fn new_by_addr_string(name: String, addr: &String) -> Result<BluetoothDevice, ()> {

        if let Some(u64_addr) = mac_string_to_u64(&addr) {
            return Ok(BluetoothDevice {
                name: name.clone(),
                addr: u64_addr,
            });
        } else {
            Err(())
        }
    }

    pub fn empty() -> BluetoothDevice {
        return BluetoothDevice::new("".to_string(), 0);
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    pub fn addr_string(&self) -> String {
        mac_u64_to_string(self.addr)
    }

}