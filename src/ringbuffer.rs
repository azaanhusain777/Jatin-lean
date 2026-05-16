//! Lock-free Single-Producer Single-Consumer (SPSC) ring buffer.
//!
//! Provides a high-performance, lock-free queue for parallel writes during
//! scanning operations. Uses cache-line alignment to prevent false sharing.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Cache-line aligned wrapper to prevent false sharing.
#[repr(align(64))]
pub struct CacheAligned<T> {
    value: T,
}

impl<T> CacheAligned<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

/// A lock-free SPSC ring buffer with cache-line aligned indices.
pub struct RingBuffer<T> {
    buffer: Vec<UnsafeCell<Option<T>>>,
    capacity: usize,
    mask: usize,
    write_idx: CacheAligned<AtomicUsize>,
    read_idx: CacheAligned<AtomicUsize>,
}

unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Send> Sync for RingBuffer<T> {}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer (capacity rounded to next power of two).
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Ring buffer capacity must be > 0");
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(None));
        }
        Self {
            buffer, capacity, mask,
            write_idx: CacheAligned::new(AtomicUsize::new(0)),
            read_idx: CacheAligned::new(AtomicUsize::new(0)),
        }
    }

    /// Try to push an item. Returns Err(item) if full.
    pub fn try_push(&self, item: T) -> Result<(), T> {
        let write = self.write_idx.value.load(Ordering::Relaxed);
        let read = self.read_idx.value.load(Ordering::Acquire);
        if write.wrapping_sub(read) >= self.capacity {
            return Err(item);
        }
        let idx = write & self.mask;
        unsafe { *self.buffer[idx].get() = Some(item); }
        self.write_idx.value.store(write.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Try to pop an item. Returns None if empty.
    pub fn try_pop(&self) -> Option<T> {
        let read = self.read_idx.value.load(Ordering::Relaxed);
        let write = self.write_idx.value.load(Ordering::Acquire);
        if read == write { return None; }
        let idx = read & self.mask;
        let item = unsafe { (*self.buffer[idx].get()).take() };
        self.read_idx.value.store(read.wrapping_add(1), Ordering::Release);
        item
    }

    pub fn is_empty(&self) -> bool {
        let read = self.read_idx.value.load(Ordering::Acquire);
        let write = self.write_idx.value.load(Ordering::Acquire);
        read == write
    }

    pub fn is_full(&self) -> bool {
        let write = self.write_idx.value.load(Ordering::Acquire);
        let read = self.read_idx.value.load(Ordering::Acquire);
        write.wrapping_sub(read) >= self.capacity
    }

    pub fn len(&self) -> usize {
        let write = self.write_idx.value.load(Ordering::Acquire);
        let read = self.read_idx.value.load(Ordering::Acquire);
        write.wrapping_sub(read)
    }

    pub fn capacity(&self) -> usize { self.capacity }

    /// Drain all items into a Vec.
    pub fn drain_all(&self) -> Vec<T> {
        let mut result = Vec::new();
        while let Some(item) = self.try_pop() {
            result.push(item);
        }
        result
    }
}

/// Multi-producer, single-consumer buffer built on crossbeam channels.
pub struct ConcurrentRingBuffer<T: Send> {
    sender: crossbeam::channel::Sender<T>,
    receiver: crossbeam::channel::Receiver<T>,
    count: AtomicUsize,
}

impl<T: Send> ConcurrentRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(capacity);
        Self { sender, receiver, count: AtomicUsize::new(0) }
    }

    pub fn unbounded() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();
        Self { sender, receiver, count: AtomicUsize::new(0) }
    }

    pub fn push(&self, item: T) -> Result<(), crossbeam::channel::SendError<T>> {
        let result = self.sender.send(item);
        if result.is_ok() { self.count.fetch_add(1, Ordering::Relaxed); }
        result
    }

    pub fn try_push(&self, item: T) -> Result<(), crossbeam::channel::TrySendError<T>> {
        let result = self.sender.try_send(item);
        if result.is_ok() { self.count.fetch_add(1, Ordering::Relaxed); }
        result
    }

    pub fn try_pop(&self) -> Option<T> {
        match self.receiver.try_recv() {
            Ok(item) => { self.count.fetch_sub(1, Ordering::Relaxed); Some(item) }
            Err(_) => None,
        }
    }

    pub fn drain_all(&self) -> Vec<T> {
        let mut result = Vec::new();
        while let Ok(item) = self.receiver.try_recv() {
            result.push(item);
            self.count.fetch_sub(1, Ordering::Relaxed);
        }
        result
    }

    pub fn len(&self) -> usize { self.count.load(Ordering::Relaxed) }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn sender(&self) -> crossbeam::channel::Sender<T> { self.sender.clone() }
}

/// Batch-oriented ring buffer that flushes in batches.
pub struct BatchRingBuffer<T: Send> {
    inner: Arc<ConcurrentRingBuffer<Vec<T>>>,
    batch_size: usize,
}

impl<T: Send> BatchRingBuffer<T> {
    pub fn new(capacity: usize, batch_size: usize) -> Self {
        Self { inner: Arc::new(ConcurrentRingBuffer::new(capacity)), batch_size }
    }

    pub fn batch_size(&self) -> usize { self.batch_size }

    pub fn flush_batch(&self, batch: Vec<T>) {
        if !batch.is_empty() { let _ = self.inner.push(batch); }
    }

    pub fn drain_all_flat(&self) -> Vec<T> {
        let batches = self.inner.drain_all();
        batches.into_iter().flatten().collect()
    }

    pub fn inner(&self) -> Arc<ConcurrentRingBuffer<Vec<T>>> { Arc::clone(&self.inner) }
}

/// Ring buffer usage statistics.
#[derive(Debug, Clone)]
pub struct RingBufferStats {
    pub total_pushed: u64,
    pub total_popped: u64,
    pub peak_occupancy: usize,
    pub push_failures: u64,
    pub capacity: usize,
}

impl RingBufferStats {
    pub fn new(capacity: usize) -> Self {
        Self { total_pushed: 0, total_popped: 0, peak_occupancy: 0, push_failures: 0, capacity }
    }
    pub fn peak_utilization_pct(&self) -> f64 {
        if self.capacity == 0 { 0.0 } else { (self.peak_occupancy as f64 / self.capacity as f64) * 100.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ringbuffer_basic() {
        let rb = RingBuffer::new(4);
        assert!(rb.is_empty());
        assert_eq!(rb.capacity(), 4);
        rb.try_push(1).unwrap();
        rb.try_push(2).unwrap();
        rb.try_push(3).unwrap();
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.try_pop(), Some(1));
        assert_eq!(rb.try_pop(), Some(2));
        assert_eq!(rb.try_pop(), Some(3));
        assert_eq!(rb.try_pop(), None);
    }

    #[test]
    fn test_ringbuffer_full() {
        let rb = RingBuffer::new(2);
        rb.try_push(1).unwrap();
        rb.try_push(2).unwrap();
        assert!(rb.is_full());
        assert!(rb.try_push(3).is_err());
    }

    #[test]
    fn test_ringbuffer_wraparound() {
        let rb = RingBuffer::new(4);
        for round in 0..10 {
            let base = round * 4;
            for i in 0..4 { rb.try_push(base + i).unwrap(); }
            for i in 0..4 { assert_eq!(rb.try_pop(), Some(base + i)); }
        }
    }

    #[test]
    fn test_concurrent_ringbuffer() {
        let crb = Arc::new(ConcurrentRingBuffer::new(1024));
        let crb2 = Arc::clone(&crb);
        let t = std::thread::spawn(move || {
            for i in 0..100 { crb2.push(i).unwrap(); }
        });
        t.join().unwrap();
        assert_eq!(crb.drain_all().len(), 100);
    }

    #[test]
    fn test_batch_ringbuffer() {
        let brb = BatchRingBuffer::<i32>::new(16, 4);
        brb.flush_batch(vec![1, 2, 3, 4]);
        brb.flush_batch(vec![5, 6, 7, 8]);
        assert_eq!(brb.drain_all_flat().len(), 8);
    }
}
