use uuid::Uuid;
use windows::{core::{GUID, Result}, Devices::Bluetooth::Rfcomm::RfcommServiceId};

pub(crate) fn create_service_id(uuid: Uuid) -> Result<RfcommServiceId> {
    let guid = GUID::from_u128(uuid.as_u128());
    RfcommServiceId::FromUuid(guid)
}