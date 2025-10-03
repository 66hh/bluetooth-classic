use std::task::Poll;

use windows::{Devices::{Bluetooth::{self}, Enumeration::{DeviceInformation, DevicePairingKinds}}, Foundation::TypedEventHandler, Networking::Sockets::StreamSocket};
use tokio::{io::{AsyncRead, AsyncWrite}, runtime::Builder, time};
use uuid::Uuid;

use crate::{common::device::{BluetoothDevice, SPP_UUID}, windows::{pair::pair_handler, utils::{winrt_async, winrt_async_action, winrt_async_with_error, winrt_error_wrap, winrt_error_wrap_with_error, winrt_none_error_wrap_with_error}, uuid::create_service_id}, BluetoothError, BluetoothSppSession};

pub struct WinrtSession {
    uuid: Uuid,
    device: BluetoothDevice,
    socket: StreamSocket,
    ready: bool
}

impl WinrtSession {
    
    pub fn new() -> WinrtSession {
        return WinrtSession {
            uuid: SPP_UUID,
            device: BluetoothDevice::empty(),
            socket: StreamSocket::new().unwrap(),
            ready: false
        };
    }

}

impl BluetoothSppSession for WinrtSession {

    fn connect(&mut self, device: &BluetoothDevice, need_pairing: bool) -> crate::Result<()> {
        self.connect_by_uuid(device, SPP_UUID, need_pairing)
    }

    fn connect_timeout(&mut self, device: &BluetoothDevice, need_pairing: bool, timeout: std::time::Duration) -> crate::Result<()> {
        self.connect_by_uuid_timeout(device, SPP_UUID, need_pairing, timeout)
    }

    fn connect_by_uuid(&mut self, device: &BluetoothDevice, uuid: Uuid, need_pairing: bool) -> crate::Result<()> {
        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            self.connect_by_uuid_async(device, uuid, need_pairing).await
        })
    }

    fn connect_by_uuid_timeout(&mut self, device: &BluetoothDevice, uuid: Uuid, need_pairing: bool, timeout: std::time::Duration) -> crate::Result<()> {

        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            time::timeout(timeout, async {
                self.connect_by_uuid_async(device, uuid, need_pairing).await
            })
            .await
        });

        if let Err(_) = result {
            return Err(BluetoothError::TimedOut(timeout));
        } else if let Ok(Err(err)) = result{
            return Err(err);
        }

        return Ok(())
    }

    async fn connect_by_uuid_async(&mut self, device: &BluetoothDevice, uuid: Uuid, need_pairing: bool) -> crate::Result<()> {

        let _ = self.socket.Close();

        self.device = device.clone();
        self.uuid = uuid;
        self.ready = false;

        // 获取查询过滤器
        let addr = self.device.addr();
        let winrt_device_filter = winrt_error_wrap(
            Bluetooth::BluetoothDevice::GetDeviceSelectorFromBluetoothAddress(addr)
        )?;

        // 查询设备
        let winrt_device_list = winrt_async_with_error(
            DeviceInformation::FindAllAsyncAqsFilter(&winrt_device_filter),
            BluetoothError::DeviceNotFound
        ).await?;

        if winrt_error_wrap_with_error(winrt_device_list.Size(), BluetoothError::DeviceNotFound)? < 1 {
            return Err(BluetoothError::DeviceNotFound);
        }

        // 获取设备信息
        let device_info = winrt_error_wrap_with_error(
            winrt_device_list.GetAt(0),
            BluetoothError::DeviceNotFound
        )?;

        // 创建设备对象
        let winrt_device = winrt_async_with_error(
            Bluetooth::BluetoothDevice::FromIdAsync(
                &winrt_error_wrap_with_error(
                    device_info.Id(),
                    BluetoothError::DeviceNotFound
                )?
            ),
            BluetoothError::DeviceNotFound
        ).await?;

        // 是否需要配对
        if need_pairing {

            // 这里要从创建的对象里重新拿一下info
            let info = winrt_error_wrap_with_error(
                winrt_device.DeviceInformation(),
                BluetoothError::DeviceNotPairing
            )?;

            let pairing = winrt_error_wrap_with_error(
                info.Pairing(),
                BluetoothError::DeviceNotPairing
            )?;

            let can_pair = winrt_error_wrap_with_error(pairing.CanPair(), BluetoothError::DeviceNotPairing)?;
            let is_paired = winrt_error_wrap_with_error(pairing.IsPaired(), BluetoothError::DeviceNotPairing)?;

            // 查询是否可配对以及是否已经配对
            if can_pair && !is_paired {
                let custom = winrt_error_wrap_with_error(
                    pairing.Custom(),
                    BluetoothError::DeviceNotPairing
                )?;

                // 弹出授权窗口
                let handler = winrt_error_wrap_with_error(
                    custom.PairingRequested(&TypedEventHandler::new(pair_handler)),
                    BluetoothError::DeviceNotPairing
                )?;

                // 配对
                winrt_async(
                    // 目前只处理直接就能配对的
                    custom.PairAsync(DevicePairingKinds::ConfirmOnly)
                ).await?;

                // 删除handler
                winrt_none_error_wrap_with_error(
                    custom.RemovePairingRequested(handler),
                    BluetoothError::DeviceNotPairing
                )?;

            }
        }
        
        // 创建服务uuid
        let service_id = winrt_error_wrap(
            create_service_id(self.uuid)
        )?;

        // 获取特定服务
        let winrt_service_list = winrt_async_with_error(
            winrt_device.GetRfcommServicesForIdAsync(&service_id),
            BluetoothError::ServiceNotFound
        ).await?;
        
        // 获取服务列表
        let list_services = winrt_error_wrap_with_error(
            winrt_service_list.Services(),
            BluetoothError::ServiceNotFound
        )?;

        if winrt_error_wrap_with_error(list_services.Size(), BluetoothError::DeviceNotFound)? < 1 {
            return Err(BluetoothError::ServiceNotFound);
        }

        // 获取服务对象
        let winrt_service = winrt_error_wrap_with_error(
            list_services.GetAt(0),
            BluetoothError::ServiceNotFound
        )?;

        // 创建socket
        self.socket = winrt_error_wrap(
            StreamSocket::new()
        )?;

        // 发起连接
        winrt_async_action(
            self.socket.ConnectAsync(&winrt_service.ConnectionHostName().unwrap(), &winrt_service.ConnectionServiceName().unwrap())
        ).await?;

        self.ready = true;

        Ok(())
    }

    async fn connect_async(&mut self, device: &BluetoothDevice, need_pairing: bool) -> crate::Result<()> {
        self.connect_by_uuid_async(device, SPP_UUID, need_pairing).await
    }

    fn device(&self) -> &BluetoothDevice {
        &self.device
    }

    fn into_device(self) -> BluetoothDevice {
        self.device
    }

}

impl AsyncRead for WinrtSession {

    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

}

impl AsyncWrite for WinrtSession {

    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

}