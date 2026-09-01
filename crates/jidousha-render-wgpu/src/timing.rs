//! How long the GPU spent on the frame, where the device will say.
//!
//! Key types: `GpuTimer`.
//! Depends on: `wgpu`, `jidousha-core` (`Seconds`). Must never depend on: a
//! clock. Nothing here reads wall time — the numbers come off the GPU's own
//! timestamp counter, which is the whole reason this reading is worth having
//! separately from the CPU-side frame breakdown (frame-pacing.md §7).
//! INVARIANT: **optional, and never load-bearing.** `TIMESTAMP_QUERY` is asked
//! for as an optional device feature (`init.rs`), so a device that does not
//! offer it is created exactly as before and this module is simply never
//! constructed. The panel prints `gpu n/a` and no frame behaves differently
//! (renderer.md §12a).
//! INVARIANT: nothing here ever blocks. The resolve is mapped asynchronously
//! and read on a later frame, so a reading is a frame or two old — which is
//! what a median over a window wants anyway, and is the opposite of
//! `capture.rs`, where blocking is the point.

use jidousha_core::Seconds;

/// How many timestamps one frame writes: the start and the end of the pass.
const TIMESTAMPS: u32 = 2;

/// The resolve buffer's size, in bytes — two 64-bit counters.
const RESOLVE_BYTES: u64 = TIMESTAMPS as u64 * 8;

/// The GPU-side stopwatch around the frame's main pass.
///
/// Constructed only on a device that granted `TIMESTAMP_QUERY`. One query set
/// and two small buffers for the life of the backend: a timer that allocated
/// per frame would be an instrument that changed what it measures.
pub(crate) struct GpuTimer {
    set: wgpu::QuerySet,
    /// Where `resolve_query_set` writes the two counters, GPU-side.
    resolve: wgpu::Buffer,
    /// The mappable copy the CPU reads them out of.
    read: wgpu::Buffer,
    /// Nanoseconds per counter tick, as the queue reports it.
    period_ns: f32,
    /// The channel a map in flight will report on, or `None` between maps.
    ///
    /// One map at a time: a second `map_async` on a buffer that is already
    /// mapped fails, and starting one every frame would make every frame's
    /// reading a failure rather than a number.
    inflight: Option<std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
    /// The last complete reading, held until a newer one lands.
    last: Option<Seconds>,
}

impl GpuTimer {
    /// A timer on a device that granted timestamp queries.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("jidousha frame timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: TIMESTAMPS,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("jidousha timestamp resolve"),
                size: RESOLVE_BYTES,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            read: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("jidousha timestamp readback"),
                size: RESOLVE_BYTES,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            period_ns: queue.get_timestamp_period(),
            inflight: None,
            last: None,
        }
    }

    /// How many bytes of buffer this timer is holding, for the accounting.
    pub(crate) fn buffer_bytes(&self) -> u64 {
        RESOLVE_BYTES * 2
    }

    /// The most recent complete reading, if one has landed yet.
    pub(crate) fn last(&self) -> Option<Seconds> {
        self.last
    }

    /// Where this frame's pass should write its two timestamps.
    ///
    /// Handed straight to the render pass descriptor. Written on **every**
    /// frame even when a map is still in flight: the counters are overwritten
    /// in place, and skipping the writes would leave the query set holding a
    /// stale pair for the resolve to pick up.
    pub(crate) fn writes(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        }
    }

    /// Queue this frame's resolve, if the last one has already been read.
    ///
    /// Called on the frame's encoder after the pass has ended and before it is
    /// submitted. A frame whose predecessor is still being mapped simply does
    /// not resolve — one reading every few frames is a median just as good and
    /// costs the frame nothing.
    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder) -> bool {
        if self.inflight.is_some() {
            return false;
        }
        encoder.resolve_query_set(&self.set, 0..TIMESTAMPS, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.read, 0, RESOLVE_BYTES);
        true
    }

    /// Start mapping the resolve this frame queued.
    ///
    /// Called after the submit, and only when [`resolve`](GpuTimer::resolve)
    /// said it had something to map.
    pub(crate) fn start_map(&mut self) {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.read
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                // A closed channel means the backend was dropped mid-frame,
                // which is a program shutting down rather than a failure.
                let _ = sender.send(result);
            });
        self.inflight = Some(receiver);
    }

    /// Take a reading if one has finished mapping.
    ///
    /// Called at the top of a frame. Polls without waiting — `PollType::Poll`
    /// services whatever is ready and returns — so a device that has not
    /// finished simply keeps the map in flight for another frame.
    pub(crate) fn collect(&mut self, device: &wgpu::Device) {
        let Some(receiver) = &self.inflight else {
            return;
        };
        // The result of the poll is deliberately ignored: a device that has
        // gone away will say so on the next `render`, which is where a caller
        // can act on it, and a diagnostic must never be the thing that reports
        // a device loss first.
        let _ = device.poll(wgpu::PollType::Poll);
        match receiver.try_recv() {
            Ok(Ok(())) => {}
            // Mapping failed, or the callback will never come. Either way this
            // timer is done: drop the map and let the next frame start a new
            // one rather than waiting forever on a reading nobody needs.
            Ok(Err(_)) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.inflight = None;
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
        }
        if let Some(counters) = self.mapped_counters() {
            self.last = elapsed(counters, self.period_ns);
        }
        self.read.unmap();
        self.inflight = None;
    }

    /// The two counters out of the mapped readback buffer.
    fn mapped_counters(&self) -> Option<[u64; TIMESTAMPS as usize]> {
        let Ok(mapped) = self.read.slice(..).get_mapped_range() else {
            return None;
        };
        let bytes: &[u8] = &mapped;
        let mut counters = [0_u64; TIMESTAMPS as usize];
        for (counter, chunk) in counters.iter_mut().zip(bytes.chunks_exact(8)) {
            let mut eight = [0_u8; 8];
            eight.copy_from_slice(chunk);
            *counter = u64::from_le_bytes(eight);
        }
        Some(counters)
    }
}

/// What two counters and a period say the pass took.
///
/// `None` rather than zero when the pair says nothing — a backend that could
/// not fill the query set leaves both counters at zero, and a wrapped or
/// out-of-order pair is the same kind of non-answer. The panel prints
/// `gpu n/a` for `None`, which is the honest reading; a zero would be a claim
/// that the GPU did the frame in no time at all.
fn elapsed(counters: [u64; TIMESTAMPS as usize], period_ns: f32) -> Option<Seconds> {
    let ticks = counters[1].checked_sub(counters[0])?;
    if ticks == 0 || !period_ns.is_finite() || period_ns <= 0.0 {
        return None;
    }
    Some(Seconds(ticks as f32 * period_ns / 1e9))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plausible period: one nanosecond a tick, which is what a Vulkan
    /// implementation with a 1GHz timestamp counter reports.
    const NANOSECOND: f32 = 1.0;

    #[test]
    fn a_pass_that_took_a_million_ticks_of_a_nanosecond_reads_as_a_millisecond() {
        let Some(seconds) = elapsed([1_000, 1_001_000], NANOSECOND) else {
            panic!("a well-formed pair has an answer");
        };
        assert!(
            (seconds.as_f32() - 0.001).abs() < 1e-9,
            "{}s is not a millisecond",
            seconds.as_f32()
        );
    }

    #[test]
    fn a_query_set_the_backend_never_filled_reads_as_no_answer_rather_than_zero() {
        // The failure this rules out: a driver that accepts the query set and
        // writes nothing into it leaves both counters at zero, and a panel
        // reporting `gpu 0.00ms` would be claiming the GPU is infinitely fast
        // on exactly the machines where the reading is unavailable.
        assert_eq!(elapsed([0, 0], NANOSECOND), None);
    }

    #[test]
    fn counters_that_went_backwards_are_not_reported_as_a_huge_frame() {
        // A wrapped counter, or a pair resolved out of order. Subtracting them
        // the other way round would produce a nineteen-digit millisecond count
        // on a panel a person is trying to read.
        assert_eq!(elapsed([1_001_000, 1_000], NANOSECOND), None);
    }

    #[test]
    fn a_device_that_reports_no_period_produces_no_reading() {
        // `get_timestamp_period` is documented to return the tick length in
        // nanoseconds; a backend with nothing to say returns zero, and
        // multiplying by it would report every frame as instant.
        assert_eq!(elapsed([0, 1_000_000], 0.0), None);
        assert_eq!(elapsed([0, 1_000_000], f32::NAN), None);
    }
}
