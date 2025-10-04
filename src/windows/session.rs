use std::{future::IntoFuture, pin::Pin, task::Poll};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    runtime::Builder,
    time,
};
use uuid::Uuid;
use windows::{
    Devices::{
        Bluetooth::{self},
        Enumeration::{DeviceInformation, DevicePairingKinds},
    },
    Foundation::TypedEventHandler,
    Networking::Sockets::StreamSocket,
    Storage::Streams::{Buffer, IBuffer, InputStreamOptions},
};

use crate::{
    BluetoothError, BluetoothSppSession,
    common::device::{BluetoothDevice, SPP_UUID},
    windows::{
        pair::pair_handler,
        utils::{
            read_input_buffer, winrt_async, winrt_async_action, winrt_async_with_error,
            winrt_error_wrap, winrt_error_wrap_with_error, winrt_none_error_wrap_with_error,
            write_output_buffer,
        },
        uuid::create_service_id,
    },
};

pub struct WinrtSession {
    uuid: Uuid,
    device: BluetoothDevice,
    socket: StreamSocket,
    ready: bool,
    // 持有正在进行的WinRT future，避免在poll中阻塞等待
    read_future: Option<Pin<Box<dyn std::future::Future<Output = windows::core::Result<IBuffer>>>>>,
    write_future: Option<Pin<Box<dyn std::future::Future<Output = windows::core::Result<u32>>>>>,
}

impl WinrtSession {
    pub fn new() -> WinrtSession {
        return WinrtSession {
            uuid: SPP_UUID,
            device: BluetoothDevice::empty(),
            socket: StreamSocket::new().unwrap(),
            ready: false,
            read_future: None,
            write_future: None,
        };
    }
}

impl BluetoothSppSession for WinrtSession {
    fn connect(&mut self, device: &BluetoothDevice, need_pairing: bool) -> crate::Result<()> {
        self.connect_by_uuid(device, SPP_UUID, need_pairing)
    }

    fn connect_timeout(
        &mut self,
        device: &BluetoothDevice,
        need_pairing: bool,
        timeout: std::time::Duration,
    ) -> crate::Result<()> {
        self.connect_by_uuid_timeout(device, SPP_UUID, need_pairing, timeout)
    }

    fn connect_by_uuid(
        &mut self,
        device: &BluetoothDevice,
        uuid: Uuid,
        need_pairing: bool,
    ) -> crate::Result<()> {
        let rt = Builder::new_multi_thread().enable_all().build().unwrap();

        rt.block_on(async { self.connect_by_uuid_async(device, uuid, need_pairing).await })
    }

    fn connect_by_uuid_timeout(
        &mut self,
        device: &BluetoothDevice,
        uuid: Uuid,
        need_pairing: bool,
        timeout: std::time::Duration,
    ) -> crate::Result<()> {
        let rt = Builder::new_multi_thread().enable_all().build().unwrap();

        let result = rt.block_on(async {
            time::timeout(timeout, async {
                self.connect_by_uuid_async(device, uuid, need_pairing).await
            })
            .await
        });

        if let Err(_) = result {
            return Err(BluetoothError::TimedOut(timeout));
        } else if let Ok(Err(err)) = result {
            return Err(err);
        }

        return Ok(());
    }

    async fn connect_by_uuid_async(
        &mut self,
        device: &BluetoothDevice,
        uuid: Uuid,
        need_pairing: bool,
    ) -> crate::Result<()> {
        let _ = self.socket.Close();

        self.device = device.clone();
        self.uuid = uuid;
        self.ready = false;
        self.read_future = None;
        self.write_future = None;

        // 获取查询过滤器
        let addr = self.device.addr();
        let winrt_device_filter = winrt_error_wrap(
            Bluetooth::BluetoothDevice::GetDeviceSelectorFromBluetoothAddress(addr),
        )?;

        // 查询设备
        let winrt_device_list = winrt_async_with_error(
            DeviceInformation::FindAllAsyncAqsFilter(&winrt_device_filter),
            BluetoothError::DeviceNotFound,
        )
        .await?;

        if winrt_error_wrap_with_error(winrt_device_list.Size(), BluetoothError::DeviceNotFound)?
            < 1
        {
            return Err(BluetoothError::DeviceNotFound);
        }

        // 获取设备信息
        let device_info = winrt_error_wrap_with_error(
            winrt_device_list.GetAt(0),
            BluetoothError::DeviceNotFound,
        )?;

        // 创建设备对象
        let winrt_device = winrt_async_with_error(
            Bluetooth::BluetoothDevice::FromIdAsync(&winrt_error_wrap_with_error(
                device_info.Id(),
                BluetoothError::DeviceNotFound,
            )?),
            BluetoothError::DeviceNotFound,
        )
        .await?;

        // 是否需要配对
        if need_pairing {
            // 这里要从创建的对象里重新拿一下info
            let info = winrt_error_wrap_with_error(
                winrt_device.DeviceInformation(),
                BluetoothError::DeviceNotPairing,
            )?;

            let pairing =
                winrt_error_wrap_with_error(info.Pairing(), BluetoothError::DeviceNotPairing)?;

            let can_pair =
                winrt_error_wrap_with_error(pairing.CanPair(), BluetoothError::DeviceNotPairing)?;
            let is_paired =
                winrt_error_wrap_with_error(pairing.IsPaired(), BluetoothError::DeviceNotPairing)?;

            // 查询是否可配对以及是否已经配对
            if can_pair && !is_paired {
                let custom = winrt_error_wrap_with_error(
                    pairing.Custom(),
                    BluetoothError::DeviceNotPairing,
                )?;

                // 弹出授权窗口
                let handler = winrt_error_wrap_with_error(
                    custom.PairingRequested(&TypedEventHandler::new(pair_handler)),
                    BluetoothError::DeviceNotPairing,
                )?;

                // 配对
                winrt_async(
                    // 目前只处理直接就能配对的
                    custom.PairAsync(DevicePairingKinds::ConfirmOnly),
                )
                .await?;

                // 删除handler
                winrt_none_error_wrap_with_error(
                    custom.RemovePairingRequested(handler),
                    BluetoothError::DeviceNotPairing,
                )?;
            }
        }

        // 创建服务uuid
        let service_id = winrt_error_wrap(create_service_id(self.uuid))?;

        // 获取特定服务
        let winrt_service_list = winrt_async_with_error(
            winrt_device.GetRfcommServicesForIdAsync(&service_id),
            BluetoothError::ServiceNotFound,
        )
        .await?;

        // 获取服务列表
        let list_services = winrt_error_wrap_with_error(
            winrt_service_list.Services(),
            BluetoothError::ServiceNotFound,
        )?;

        if winrt_error_wrap_with_error(list_services.Size(), BluetoothError::DeviceNotFound)? < 1 {
            return Err(BluetoothError::ServiceNotFound);
        }

        // 获取服务对象
        let winrt_service =
            winrt_error_wrap_with_error(list_services.GetAt(0), BluetoothError::ServiceNotFound)?;

        // 创建socket
        self.socket = winrt_error_wrap(StreamSocket::new())?;

        // 发起连接
        winrt_async_action(self.socket.ConnectAsync(
            &winrt_service.ConnectionHostName().unwrap(),
            &winrt_service.ConnectionServiceName().unwrap(),
        ))
        .await?;

        self.ready = true;

        Ok(())
    }

    async fn connect_async(
        &mut self,
        device: &BluetoothDevice,
        need_pairing: bool,
    ) -> crate::Result<()> {
        self.connect_by_uuid_async(device, SPP_UUID, need_pairing)
            .await
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
        let self_mut = self.get_mut();

        // 如果连接未准备好，直接踹踹包然后返回Pending并清理旧future
        if !self_mut.ready {
            self_mut.read_future = None;
            return Poll::Pending;
        }

        // 缓冲区没有可写空间，则认为本次读取已经完成
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        // 没有挂起的读future时，发起新的ra请求
        if self_mut.read_future.is_none() {
            let stream = match self_mut.socket.InputStream() {
                Ok(s) => s,
                Err(_) => {
                    // 获取输入流失败，标记会话未就绪然后给Pending
                    self_mut.ready = false;
                    return Poll::Pending;
                }
            };

            let cap = buf.remaining() as u32;
            let buffer = match Buffer::Create(cap) {
                Ok(b) => b,
                Err(_) => {
                    // 缓冲区创建失败，也给Pending
                    self_mut.ready = false;
                    return Poll::Pending;
                }
            };

            // 把IAsyncOperation生锈成Future并缓存下来
            self_mut.read_future = match stream.ReadAsync(&buffer, cap, InputStreamOptions::Partial)
            {
                Ok(op) => {
                    let buffer_clone = buffer.clone();
                    Some(Box::pin(async move {
                        // 打个flag，确保WinRT缓冲区在future完成前不被释放
                        let _keep_alive = buffer_clone;
                        op.into_future().await
                    }))
                }
                Err(_) => {
                    // 发起异步读取失败，等待上层重新触发
                    self_mut.ready = false;
                    return Poll::Pending;
                }
            };
        }

        // 推动挂起的future，完成后把数据写回tokio的ReadBuf
        if let Some(future) = self_mut.read_future.as_mut() {
            // 呃这其实应该就是一种嵌套poll
            match future.as_mut().poll(cx) {
                // WinRT成功返回数据，拷贝到上层缓冲区
                Poll::Ready(Ok(buffer)) => {
                    self_mut.read_future = None;
                    match read_input_buffer(buffer) {
                        Ok(vec) => {
                            // 将WinRT缓冲区内容拷贝到调用者提供的缓冲区
                            buf.put_slice(&vec);
                            return Poll::Ready(Ok(()));
                        }
                        Err(_) => {
                            self_mut.ready = false;
                            return Poll::Pending;
                        }
                    }
                }
                // WinRT future报错，重置状态等待下一次调用
                Poll::Ready(Err(_)) => {
                    self_mut.read_future = None;
                    self_mut.ready = false;
                    return Poll::Pending;
                }
                // 仍然未完成，返回Pending继续等待
                // 这就和block_on一样实现阻塞逻辑了
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        Poll::Pending
    }
}

impl AsyncWrite for WinrtSession {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let self_mut = self.get_mut();

        // 这一堆狗屎逻辑和上面的read一样
        if !self_mut.ready {
            self_mut.write_future = None;
            return Poll::Pending;
        }

        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        if self_mut.write_future.is_none() {
            let stream = match self_mut.socket.OutputStream() {
                Ok(s) => s,
                Err(_) => {
                    self_mut.ready = false;
                    return Poll::Pending;
                }
            };

            // 数据转IBuffer
            let buffer = match write_output_buffer(buf.to_vec()) {
                Ok(b) => b,
                Err(_) => {
                    self_mut.ready = false;
                    return Poll::Pending;
                }
            };

            self_mut.write_future = match stream.WriteAsync(&buffer) {
                Ok(op) => {
                    let buffer_clone = buffer.clone();
                    Some(Box::pin(async move {
                        // poll同款keep-alive
                        let _keep_alive = buffer_clone;
                        op.into_future().await
                    }))
                }
                Err(_) => {
                    self_mut.ready = false;
                    return Poll::Pending;
                }
            };
        }

        if let Some(future) = self_mut.write_future.as_mut() {
            match future.as_mut().poll(cx) {
                Poll::Ready(Ok(written)) => {
                    self_mut.write_future = None;
                    return Poll::Ready(Ok(written as usize));
                }
                Poll::Ready(Err(_)) => {
                    self_mut.write_future = None;
                    self_mut.ready = false;
                    return Poll::Pending;
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        Poll::Pending
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}
