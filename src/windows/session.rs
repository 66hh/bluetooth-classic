use std::task::Poll;

use windows::{Devices::Bluetooth::{self}, Networking::Sockets::StreamSocket};
use tokio::{io::{AsyncRead, AsyncWrite}, runtime::Builder, time};
use uuid::Uuid;

use crate::{common::device::{BluetoothDevice, SPP_UUID}, windows::{utils::{winrt_async_action, winrt_async_with_error, winrt_error_wrap, winrt_error_wrap_with_error}, uuid::create_service_id}, BluetoothError, BluetoothSppSession};

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

    fn connect(&mut self, device: &BluetoothDevice) -> crate::Result<()> {
        self.connect_by_uuid(device, SPP_UUID)
    }

    fn connect_timeout(&mut self, device: &BluetoothDevice, timeout: std::time::Duration) -> crate::Result<()> {
        self.connect_by_uuid_timeout(device, SPP_UUID, timeout)
    }

    fn connect_by_uuid(&mut self, device: &BluetoothDevice, uuid: Uuid) -> crate::Result<()> {
        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            self.connect_by_uuid_async(device, uuid).await
        })
    }

    fn connect_by_uuid_timeout(&mut self, device: &BluetoothDevice, uuid: Uuid, timeout: std::time::Duration) -> crate::Result<()> {

        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            time::timeout(timeout, async {
                self.connect_by_uuid_async(device, uuid).await
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

    async fn connect_by_uuid_async(&mut self, device: &BluetoothDevice, uuid: Uuid) -> crate::Result<()> {

        let _ = self.socket.Close();

        self.device = device.clone();
        self.uuid = uuid;
        self.ready = false;

        let addr = self.device.addr();
        let winrt_device = winrt_async_with_error(
            Bluetooth::BluetoothDevice::FromBluetoothAddressAsync(addr),
            BluetoothError::DeviceNotFound
        ).await?;

        let service_id = winrt_error_wrap(
            create_service_id(self.uuid)
        )?;

        let winrt_service_list = winrt_async_with_error(
            winrt_device.GetRfcommServicesForIdAsync(&service_id),
            BluetoothError::ServiceNotFound
        ).await?;
        
        let list_services = winrt_error_wrap_with_error(
            winrt_service_list.Services(),
            BluetoothError::ServiceNotFound
        )?;

        let service_result = winrt_error_wrap_with_error(
            list_services.First(),
            BluetoothError::ServiceNotFound
        )?;


        let winrt_service = winrt_error_wrap_with_error(
            service_result.Current(),
            BluetoothError::ServiceNotFound
        )?;

        self.socket = winrt_error_wrap(
            StreamSocket::new()
        )?;

        winrt_async_action(
            self.socket.ConnectAsync(&winrt_service.ConnectionHostName().unwrap(), &winrt_service.ConnectionServiceName().unwrap())
        ).await?;

        self.ready = true;

        Ok(())
    }

    async fn connect_async(&mut self, device: &BluetoothDevice) -> crate::Result<()> {
        self.connect_by_uuid_async(device, SPP_UUID).await
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