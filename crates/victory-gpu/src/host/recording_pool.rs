//! Command storage shared by bounded groups of main-list recordings.
//!
//! A lease exclusively owns recording until Close, then keeps those commands
//! intact until the host list is retired. Other closed lists may share the
//! allocator, including while the GPU executes them. Reset requires both no
//! leases and completion of the last signal on this Host's direct queue.

use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use windows::Win32::Graphics::Direct3D12::*;

pub(super) fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| crate::switch::on("VICTORY_SHARED_RECORDING_ALLOCATORS"))
}

const RECORDINGS: u32 = 32;

#[derive(Default)]
pub(super) struct Pool {
    batches: Mutex<Vec<Arc<Batch>>>,
    shared: AtomicU64,
    resets: AtomicU64,
}

struct Batch {
    allocator: ID3D12CommandAllocator,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    recording: bool,
    leases: u32,
    recordings: u32,
    after: u64,
}

pub(super) struct Lease {
    batch: Arc<Batch>,
    recording: bool,
}

impl Pool {
    /// `fence` must be the single queue fence belonging to this pool's Host.
    pub fn take(&self, device: &ID3D12Device, fence: &ID3D12Fence) -> windows::core::Result<Lease> {
        // SAFETY: querying a live fence does not change it.
        let completed = unsafe { fence.GetCompletedValue() };
        if completed == u64::MAX {
            return Err(windows::core::Error::from_hresult(windows::Win32::Graphics::Dxgi::DXGI_ERROR_DEVICE_REMOVED));
        }
        let mut batches = self.batches.lock().unwrap_or_else(|e| e.into_inner());
        for batch in batches.iter() {
            let mut state = batch.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.recording {
                continue;
            }
            if state.leases == 0 && state.after <= completed {
                // SAFETY: no list retains a replayable recording, no list
                // records now, and the queue has completed every execution.
                unsafe { batch.allocator.Reset()? };
                *state = State::default();
                self.resets.fetch_add(1, Relaxed);
            } else if state.recordings >= RECORDINGS {
                continue;
            } else {
                self.shared.fetch_add(1, Relaxed);
            }
            state.recording = true;
            state.leases += 1;
            state.recordings += 1;
            return Ok(Lease { batch: Arc::clone(batch), recording: true });
        }
        // SAFETY: ordinary creation; only this new lease can record with it.
        let allocator = unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)? };
        let batch = Arc::new(Batch {
            allocator,
            state: Mutex::new(State { recording: true, leases: 1, recordings: 1, after: 0 }),
        });
        batches.push(Arc::clone(&batch));
        Ok(Lease { batch, recording: true })
    }

    pub fn log(&self) {
        if !on() { return; }
        let batches = self.batches.lock().unwrap_or_else(|e| e.into_inner());
        let (mut recording, mut leases) = (0, 0);
        for batch in batches.iter() {
            let state = batch.state.lock().unwrap_or_else(|e| e.into_inner());
            recording += u32::from(state.recording);
            leases += state.leases;
        }
        tracing::info!(allocators = batches.len(), recording, leases, batch_limit = RECORDINGS,
            shared = self.shared.load(Relaxed), resets = self.resets.load(Relaxed),
            "main recording allocator groups (reset after leases and queue fence)");
    }
}

impl Lease {
    pub fn allocator(&self) -> &ID3D12CommandAllocator { &self.batch.allocator }

    pub fn is_recording(&self) -> bool { self.recording }

    /// Only after the native list has successfully closed.
    pub fn close(&mut self) {
        if self.recording {
            self.batch.state.lock().unwrap_or_else(|e| e.into_inner()).recording = false;
            self.recording = false;
        }
    }

    /// The signal following this list's latest execution on the Host queue.
    /// Keeping the lease also prevents reset of an unsubmitted/replayable list.
    pub fn submitted(&self, after: u64) {
        debug_assert!(!self.recording);
        let mut state = self.batch.state.lock().unwrap_or_else(|e| e.into_inner());
        state.after = state.after.max(after);
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        // An abandoned open list must be closed or destroyed before its
        // lease is dropped. HostList declares its lease after the native list.
        let mut state = self.batch.state.lock().unwrap_or_else(|e| e.into_inner());
        if self.recording { state.recording = false; }
        state.leases -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::Interface;

    #[test]
    #[ignore = "native allocator sharing, replay and queue-fence lifetime test"]
    fn recordings_share_only_after_close_and_reset_only_after_retirement_and_fence() {
        let host = crate::host::host().expect("native GPU");
        let pool = Pool::default();
        // SAFETY: resources and command lists remain alive through completion.
        unsafe {
            let queue: ID3D12CommandQueue = host.device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT, ..Default::default()
            }).unwrap();
            let gate: ID3D12Fence = host.device.CreateFence(0, D3D12_FENCE_FLAG_NONE).unwrap();
            let done: ID3D12Fence = host.device.CreateFence(0, D3D12_FENCE_FLAG_NONE).unwrap();
            let (upload, data) = host.upload_buffer(256).unwrap();
            let back = host.read_back_buffer(256).unwrap();
            for i in 0..RECORDINGS { data.cast::<u32>().add(i as usize).write(17 + 13 * i); }
            let mut first = pool.take(&host.device, &done).unwrap();
            let first_id = first.allocator().as_raw();
            let list: ID3D12GraphicsCommandList = host.device.CreateCommandList(0,
                D3D12_COMMAND_LIST_TYPE_DIRECT, first.allocator(), None).unwrap();
            let separate = pool.take(&host.device, &done).unwrap();
            assert_ne!(first_id, separate.allocator().as_raw(), "open recordings shared storage");
            list.CopyBufferRegion(&back, 0, &upload, 0, 4);
            list.Close().unwrap();
            first.close();
            let mut lists: Vec<Option<ID3D12CommandList>> = vec![Some(list.cast().unwrap())];
            let mut leases = vec![first];
            for i in 1..RECORDINGS {
                let mut lease = pool.take(&host.device, &done).unwrap();
                assert_eq!(lease.allocator().as_raw(), first_id, "closed recordings did not share");
                let list: ID3D12GraphicsCommandList = host.device.CreateCommandList(0,
                    D3D12_COMMAND_LIST_TYPE_DIRECT, lease.allocator(), None).unwrap();
                list.CopyBufferRegion(&back, u64::from(i) * 4, &upload, u64::from(i) * 4, 4);
                list.Close().unwrap();
                lease.close();
                lists.push(Some(list.cast().unwrap()));
                leases.push(lease);
            }
            queue.ExecuteCommandLists(&lists);
            queue.Signal(&done, 1).unwrap();
            for lease in &leases { lease.submitted(1); }
            done.SetEventOnCompletion(1, windows::Win32::Foundation::HANDLE::default()).unwrap();
            let retained = pool.take(&host.device, &done).unwrap();
            assert_ne!(retained.allocator().as_raw(), first_id, "a replayable recording was reset");
            // Re-execute while blocked, release the CPU leases, and verify
            // that the queue fence independently prevents allocator reset.
            queue.Wait(&gate, 1).unwrap();
            queue.ExecuteCommandLists(&lists);
            queue.Signal(&done, 2).unwrap();
            for lease in &leases { lease.submitted(2); }
            drop(leases);
            let waiting = done.GetCompletedValue();
            let blocked = pool.take(&host.device, &done);
            // Unblock before assertions so a failure cannot strand the queue.
            gate.Signal(1).unwrap();
            done.SetEventOnCompletion(2, windows::Win32::Foundation::HANDLE::default()).unwrap();
            let blocked = blocked.unwrap();
            assert_eq!(waiting, 1);
            assert_ne!(blocked.allocator().as_raw(), first_id, "unfinished commands were reset");
            let mut reused = pool.take(&host.device, &done).unwrap();
            assert_eq!(reused.allocator().as_raw(), first_id);
            list.Reset(reused.allocator(), None).unwrap();
            list.CopyBufferRegion(&back, 0, &upload, 0, u64::from(RECORDINGS) * 4);
            list.Close().unwrap();
            reused.close();
            queue.ExecuteCommandLists(&[Some(list.cast().unwrap())]);
            queue.Signal(&done, 3).unwrap();
            reused.submitted(3);
            done.SetEventOnCompletion(3, windows::Win32::Foundation::HANDLE::default()).unwrap();
            let mut mapped = std::ptr::null_mut();
            back.Map(0, None, Some(&mut mapped)).unwrap();
            let values = std::slice::from_raw_parts(mapped.cast::<u32>(), RECORDINGS as usize);
            assert_eq!(values, (0..RECORDINGS).map(|i| 17 + 13 * i).collect::<Vec<_>>());
            back.Unmap(0, None);
            assert_eq!(pool.resets.load(Relaxed), 1);
            assert!(host.device.GetDeviceRemovedReason().is_ok());
            assert_eq!(host.log_messages(), None, "D3D12 rejected shared command storage");
            drop((separate, retained, blocked));
        }
    }
}
