use windows::{core, Storage::Streams::{DataReader, DataWriter, IBuffer}};

use crate::BluetoothError;

pub fn winrt_error_wrap<T: core::RuntimeType + 'static>(result: core::Result<T>) -> crate::Result<T> {
    match result {
        Ok(res) => return Ok(res),
        Err(err) => return Err(BluetoothError::RuntimeError(err.to_string())),
    }
}

pub fn winrt_none_error_wrap(result: core::Result<()>) -> crate::Result<()> {
    match result {
        Ok(_) => return Ok(()),
        Err(err) => return Err(BluetoothError::RuntimeError(err.to_string())),
    }
}

pub fn winrt_none_error_wrap_with_error(result: core::Result<()>, error: BluetoothError) -> crate::Result<()> {
    match result {
        Ok(_) => return Ok(()),
        Err(_) => return Err(error),
    }
}

pub fn winrt_error_wrap_with_error<T: core::RuntimeType + 'static>(result: core::Result<T>, error: BluetoothError) -> crate::Result<T> {
    match result {
        Ok(res) => return Ok(res),
        Err(_) => return Err(error),
    }
}

pub async fn winrt_async<T: core::RuntimeType + 'static>(result: core::Result<windows_future::IAsyncOperation<T>>) -> crate::Result<T> {
    match result {
        Ok(op) => {
            match op.await {
                Ok(res) => return Ok(res),
                Err(err) => return Err(BluetoothError::RuntimeError(err.to_string())),
            }
        },
        Err(err) => return Err(BluetoothError::RuntimeError(err.to_string())),
    }
}

pub async fn winrt_async_with_error<T: core::RuntimeType + 'static>(result: core::Result<windows_future::IAsyncOperation<T>>, error: BluetoothError) -> crate::Result<T> {
    match result {
        Ok(op) => {
            match op.await {
                Ok(res) => return Ok(res),
                Err(_) => return Err(error),
            }
        },
        Err(_) => return Err(error),
    }
}

pub async fn winrt_async_action(result: core::Result<windows_future::IAsyncAction>) -> crate::Result<()> {
    match result {
        Ok(op) => {
            match op.await {
                Ok(_) => return Ok(()),
                Err(err) => return Err(BluetoothError::RuntimeError(err.to_string())),
            }
        },
        Err(err) => return Err(BluetoothError::RuntimeError(err.to_string())),
    }
}

pub async fn winrt_async_action_with_error(result: core::Result<windows_future::IAsyncAction>, error: BluetoothError) -> crate::Result<()> {
    match result {
        Ok(op) => {
            match op.await {
                Ok(_) => return Ok(()),
                Err(_) => return Err(error),
            }
        },
        Err(_) => return Err(error),
    }
}

pub fn read_input_buffer(buffer: IBuffer) -> core::Result<Vec<u8>> {
    let reader = DataReader::FromBuffer(&buffer)?;
    let len = reader.UnconsumedBufferLength()? as usize;
    let mut value = vec![0; len];
    reader.ReadBytes(value.as_mut_slice())?;
    Ok(value.to_vec())
}

pub fn write_output_buffer(bytes: Vec<u8>) -> core::Result<IBuffer> {
    let writer = DataWriter::new()?;
    writer.WriteBytes(&bytes)?;
    writer.DetachBuffer()
}