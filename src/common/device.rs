use uuid::{uuid, Uuid};

pub static SPP_UUID: Uuid = uuid!("00001101-0000-1000-8000-00805F9B34FB");

#[derive(Clone)]
pub struct BluetoothDevice {
    pub name: String,
    pub addr: String,
}

impl BluetoothDevice {
    
    pub fn new(name: String, addr: String) -> BluetoothDevice {
        return BluetoothDevice {
            name: name.clone(),
            addr: addr.clone()
        };
    }

    pub fn empty() -> BluetoothDevice {
        return BluetoothDevice::new("".to_string(), "".to_string());
    }

    pub fn name(self) -> String {
        self.name.clone()
    }

    pub fn addr(self) -> String {
        self.addr.clone()
    }

}