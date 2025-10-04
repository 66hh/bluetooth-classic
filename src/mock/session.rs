use std::{task::Poll, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    runtime::Builder,
    time::{self, sleep},
};
use uuid::Uuid;

use crate::{BluetoothDevice, BluetoothError, BluetoothSppSession, common::device::SPP_UUID};

pub struct MockSession {
    uuid: Uuid,
    device: BluetoothDevice,
    need_pairing: bool,
    blocked: bool,
    buffer: Vec<u8>,
    position: usize,
    is_ready: bool,
}

impl MockSession {
    pub fn new() -> MockSession {
        return MockSession {
            uuid: SPP_UUID,
            device: BluetoothDevice::empty(),
            need_pairing: true,
            blocked: false,
            buffer: Vec::new(),
            position: 0,
            is_ready: false,
        };
    }

    pub fn blocked_connect(&mut self, blocked: bool) {
        self.blocked = blocked;
    }
}

impl BluetoothSppSession for MockSession {
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
        self.device = device.clone();
        self.uuid = uuid;
        self.need_pairing = need_pairing;

        while self.blocked {
            sleep(Duration::from_millis(10)).await;
        }

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

impl AsyncRead for MockSession {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();

        if self_mut.is_ready {
            let data = &self_mut.buffer[self_mut.position..];
            buf.put_slice(data);
            self_mut.position += data.len();
            Poll::Ready(Ok(()))
        } else {
            self_mut.is_ready = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl AsyncWrite for MockSession {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let self_mut = self.get_mut();
        self_mut.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let self_mut = self.get_mut();

        if self_mut.is_ready {
            Poll::Ready(Ok(()))
        } else {
            self_mut.is_ready = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}
