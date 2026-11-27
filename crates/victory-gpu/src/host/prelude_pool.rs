//! One allocator for the serially recorded state preludes of a submission.
//! A returned allocator carries the host queue fence value after every list
//! recorded into it. Only that fence's completion permits its next reset.

use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use windows::Win32::Graphics::Direct3D12::*;

pub(super) fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| crate::switch::on("VICTORY_SHARED_PRELUDE_ALLOCATORS"))
}

#[derive(Default)]
pub(super) struct Pool {
    // Each pool belongs to one Host and its one submission fence.
    pending: Mutex<Vec<(u64, ID3D12CommandAllocator)>>,
    made: AtomicU64,
    reused: AtomicU64,
}

pub(super) struct Batch {
    pub allocator: ID3D12CommandAllocator,
}

impl Pool {
    /// `completed` is this host's queue fence, never a CPU submission count.
    pub fn take(&self, device: &ID3D12Device, completed: u64) -> windows::core::Result<Batch> {
        if completed == u64::MAX {
            return Err(windows::core::Error::from_hresult(windows::Win32::Graphics::Dxgi::DXGI_ERROR_DEVICE_REMOVED));
        }
        let ready = {
            let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            pending.iter().position(|(after, _)| *after <= completed).map(|i| pending.swap_remove(i).1)
        };
        let allocator = match ready {
            Some(allocator) => {
                // SAFETY: all its lists were closed before return, and the
                // queue fence following all their executions has completed.
                unsafe { allocator.Reset()? };
                self.reused.fetch_add(1, Relaxed);
                allocator
            }
            None => {
                // SAFETY: ordinary allocator creation on the live device.
                let allocator = unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)? };
                self.made.fetch_add(1, Relaxed);
                allocator
            }
        };
        Ok(Batch { allocator })
    }

    /// Called only after all batch lists have been submitted and a signal
    /// after them has been queued. No list in the batch is recorded again.
    pub fn submitted(&self, batch: Batch, after: u64) {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).push((after, batch.allocator));
    }

    pub fn log(&self) {
        if on() {
            tracing::info!(made = self.made.load(Relaxed), reused = self.reused.load(Relaxed),
                held = self.pending.lock().unwrap_or_else(|e| e.into_inner()).len(),
                "submission prelude allocators (reused only after the queue fence)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::Interface;

    #[test]
    #[ignore = "native queue/fence lifetime test; run explicitly without a game"]
    fn in_flight_allocator_is_not_reset_and_completed_batch_can_be_reused() {
        let host = crate::host::host().expect("native GPU");
        let pool = Pool::default();
        // SAFETY: live device objects, kept until their queue fence completes.
        unsafe {
            let queue: ID3D12CommandQueue = host.device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT, ..Default::default()
            }).unwrap();
            let gate: ID3D12Fence = host.device.CreateFence(0, D3D12_FENCE_FLAG_NONE).unwrap();
            let done: ID3D12Fence = host.device.CreateFence(0, D3D12_FENCE_FLAG_NONE).unwrap();
            let (upload, data) = host.upload_buffer(256).unwrap();
            let back = host.read_back_buffer(256).unwrap();
            for (i, value) in [17u32, 29, 43, 71].into_iter().enumerate() {
                data.cast::<u32>().add(i).write(value);
            }
            let batch = pool.take(&host.device, done.GetCompletedValue()).unwrap();
            let first = batch.allocator.as_raw();
            let mut lists: Vec<Option<ID3D12CommandList>> = Vec::new();
            for index in 0..2u64 {
                let list: ID3D12GraphicsCommandList = host.device.CreateCommandList(0,
                    D3D12_COMMAND_LIST_TYPE_DIRECT, &batch.allocator, None).unwrap();
                list.CopyBufferRegion(&back, index * 4, &upload, index * 4, 4);
                list.Close().unwrap();
                lists.push(Some(list.cast().unwrap()));
            }
            queue.Wait(&gate, 1).unwrap();
            queue.ExecuteCommandLists(&lists);
            queue.Signal(&done, 1).unwrap();
            pool.submitted(batch, 1);
            let still_waiting = done.GetCompletedValue();
            let second = pool.take(&host.device, still_waiting).unwrap();
            let other = second.allocator.as_raw();
            // Release the queue before assertions, so a failing assertion
            // does not leave a native queue stalled indefinitely.
            gate.Signal(1).unwrap();
            done.SetEventOnCompletion(1, windows::Win32::Foundation::HANDLE::default()).unwrap();
            assert_eq!(still_waiting, 0);
            assert_ne!(first, other, "an unfinished allocator was reused");
            let reused = pool.take(&host.device, done.GetCompletedValue()).unwrap();
            assert_eq!(reused.allocator.as_raw(), first);
            let list: ID3D12GraphicsCommandList = host.device.CreateCommandList(0,
                D3D12_COMMAND_LIST_TYPE_DIRECT, &reused.allocator, None).unwrap();
            list.CopyBufferRegion(&back, 8, &upload, 8, 8);
            list.Close().unwrap();
            queue.ExecuteCommandLists(&[Some(list.cast().unwrap())]);
            queue.Signal(&done, 2).unwrap();
            pool.submitted(reused, 2);
            done.SetEventOnCompletion(2, windows::Win32::Foundation::HANDLE::default()).unwrap();
            let mut mapped = std::ptr::null_mut();
            back.Map(0, None, Some(&mut mapped)).unwrap();
            let values = std::slice::from_raw_parts(mapped.cast::<u32>(), 4);
            assert_eq!(values, [17, 29, 43, 71]);
            back.Unmap(0, None);
            assert!(pool.take(&host.device, u64::MAX).is_err());
            assert!(host.device.GetDeviceRemovedReason().is_ok());
        }
    }
}
