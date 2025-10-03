use windows::{core::Ref, Devices::Enumeration::{DeviceInformationCustomPairing, DevicePairingKinds, DevicePairingRequestedEventArgs}};

pub fn pair_handler(
    _pairing: Ref<'_, DeviceInformationCustomPairing>,
    args: Ref<'_, DevicePairingRequestedEventArgs>,
) -> windows::core::Result<()> {

    if let Some(args) = args.as_ref() {
        match args.PairingKind()? {

            // 目前只处理直接就能配对的
            DevicePairingKinds::ConfirmOnly => args.Accept()?,

            // TODO
            _ => args.Accept()?,
        }
    }

    Ok(())
}