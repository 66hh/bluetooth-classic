use uuid::Uuid;
use windows::{
    Devices::Bluetooth::Rfcomm::RfcommServiceId,
    core::{GUID, Result},
};

pub(crate) fn create_service_id(uuid: Uuid) -> Result<RfcommServiceId> {
    let guid = GUID::from_u128(uuid.as_u128());
    RfcommServiceId::FromUuid(guid)
}
