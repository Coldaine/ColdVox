# Phase 2: Lock-free Ring Buffer Implementation Design

**Created:** 2025-08-24  
**Status:** Design Phase

## Executive Summary

This document provides the detailed implementation design for replacing the crossbeam channel with a production-grade lock-free ring buffer for real-time audio processing in ColdVox.

## Core Design Principles

1. **Zero Allocations in Audio Path**: Pre-allocated memory, no dynamic allocation during audio callback
2. **Cache-Friendly Layout**: Contiguous memory with power-of-2 sizing for efficient indexing
3. **Lock-Free Operation**: Atomic operations with careful memory ordering
4. **Single Producer, Single Consumer (SPSC)**: Optimized for audio callback → processing thread model
5. **Bounded Buffer**: Fixed capacity with configurable overflow policies

## Data Structure Design

### Core Ring Buffer

```rust
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::sync::Arc;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

/// Lock-free SPSC ring buffer for audio samples
pub struct AudioRingBuffer {
    /// Pre-allocated sample storage (power of 2 size)
    buffer: *mut i16,
    
    /// Total capacity in samples (must be power of 2)
    capacity: usize,
    
    /// Mask for fast modulo operation (capacity - 1)
    mask: usize,
    
    /// Write position (only modified by producer)
    write_pos: AtomicUsize,
    
    /// Read position (only modified by consumer)  
    read_pos: AtomicUsize,
    
    /// Cached write position (producer-local optimization)
    cached_write_pos: usize,
    
    /// Cached read position (consumer-local optimization)
    cached_read_pos: usize,
    
    /// Sequence number for continuity tracking
    sequence: AtomicU64,
    
    /// Overflow policy configuration
    overflow_policy: OverflowPolicy,
    
    /// Statistics
    stats: BufferStats,
}

#[derive(Debug, Clone, Copy)]
pub enum OverflowPolicy {
    /// Drop incoming samples when buffer is full (default)
    DropNewest,
    /// Overwrite oldest samples when buffer is full
    DropOldest,
    /// Panic on overflow (debug only)
    Panic,
}

pub struct BufferStats {
    samples_written: AtomicU64,
    samples_dropped: AtomicU64,
    overflow_events: AtomicU64,
    underflow_events: AtomicU64,
    max_utilization: AtomicUsize,
}
```

### Metadata Tracking

```rust
/// Metadata ring buffer (parallel to sample buffer)
pub struct MetadataRingBuffer {
    /// Frame boundaries in the sample buffer
    frame_boundaries: Vec<FrameBoundary>,
    
    /// Write index for frame boundaries
    write_idx: AtomicUsize,
    
    /// Read index for frame boundaries  
    read_idx: AtomicUsize,
}

#[derive(Clone, Copy, Debug)]
pub struct FrameBoundary {
    /// Starting sample index in main buffer
    start_idx: usize,
    
    /// Number of samples in this frame
    sample_count: usize,
    
    /// Timestamp when frame was captured
    timestamp: std::time::Instant,
    
    /// Sequence number for continuity
    seq_no: u64,
    
    /// Sample rate (for format tracking)
    sample_rate: u32,
}
```

## Memory Layout & Allocation

### Allocation Strategy

```rust
impl AudioRingBuffer {
    pub fn new(capacity_samples: usize, policy: OverflowPolicy) -> Self {
        // Ensure power of 2 for efficient masking
        let capacity = capacity_samples.next_power_of_two();
        let mask = capacity - 1;
        
        // Allocate aligned memory for cache efficiency
        let layout = Layout::from_size_align(
            capacity * std::mem::size_of::<i16>(),
            64  // Cache line alignment
        ).unwrap();
        
        let buffer = unsafe {
            let ptr = alloc(layout) as *mut i16;
            // Initialize to silence (zeros)
            ptr::write_bytes(ptr, 0, capacity);
            ptr
        };
        
        Self {
            buffer,
            capacity,
            mask,
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            cached_write_pos: 0,
            cached_read_pos: 0,
            sequence: AtomicU64::new(0),
            overflow_policy: policy,
            stats: BufferStats::new(),
        }
    }
}
```

### Memory Layout Optimization

```
┌─────────────────────────────────────────────────────┐
│ Cache Line 1 (64 bytes)                              │
├─────────────────────────────────────────────────────┤
│ write_pos (8) │ cached_write_pos (8) │ padding (48) │
├─────────────────────────────────────────────────────┤
│ Cache Line 2 (64 bytes) - False sharing prevention   │
├─────────────────────────────────────────────────────┤
│ read_pos (8)  │ cached_read_pos (8)  │ padding (48) │
├─────────────────────────────────────────────────────┤
│ Sample Buffer (aligned, contiguous)                  │
│ [i16, i16, i16, ... capacity samples]               │
└─────────────────────────────────────────────────────┘
```

## Lock-Free Algorithms

### Producer (Audio Callback) Operations

```rust
impl AudioRingBuffer {
    /// Write samples from audio callback (lock-free, wait-free)
    pub fn write_samples(&mut self, samples: &[i16]) -> Result<(), BufferError> {
        let n = samples.len();
        
        // Fast path: check cached positions first
        let write = self.cached_write_pos;
        let available = self.capacity - (write - self.cached_read_pos);
        
        if available < n {
            // Slow path: reload read position
            self.cached_read_pos = self.read_pos.load(Ordering::Acquire);
            let available = self.capacity - (write - self.cached_read_pos);
            
            if available < n {
                return self.handle_overflow(samples);
            }
        }
        
        // Copy samples to buffer (can wrap around)
        let write_idx = write & self.mask;
        let first_part = (self.capacity - write_idx).min(n);
        
        unsafe {
            // First part (until end of buffer)
            ptr::copy_nonoverlapping(
                samples.as_ptr(),
                self.buffer.add(write_idx),
                first_part
            );
            
            // Second part (wrap to beginning)
            if first_part < n {
                ptr::copy_nonoverlapping(
                    samples.as_ptr().add(first_part),
                    self.buffer,
                    n - first_part
                );
            }
        }
        
        // Update positions with release ordering
        self.cached_write_pos = write + n;
        self.write_pos.store(self.cached_write_pos, Ordering::Release);
        
        // Update stats
        self.stats.samples_written.fetch_add(n as u64, Ordering::Relaxed);
        self.sequence.fetch_add(1, Ordering::Relaxed);
        
        Ok(())
    }
    
    fn handle_overflow(&mut self, samples: &[i16]) -> Result<(), BufferError> {
        match self.overflow_policy {
            OverflowPolicy::DropNewest => {
                self.stats.samples_dropped.fetch_add(samples.len() as u64, Ordering::Relaxed);
                self.stats.overflow_events.fetch_add(1, Ordering::Relaxed);
                Err(BufferError::Overflow)
            }
            OverflowPolicy::DropOldest => {
                // Force advance read pointer
                let skip = samples.len();
                self.read_pos.fetch_add(skip, Ordering::AcqRel);
                self.write_samples(samples)
            }
            OverflowPolicy::Panic => {
                panic!("Audio buffer overflow with {} samples", samples.len());
            }
        }
    }
}
```

### Consumer (Processing Thread) Operations

```rust
impl AudioRingBuffer {
    /// Read exactly `count` samples, padding with zeros if needed
    pub fn read_samples(&mut self, count: usize, buffer: &mut [i16]) -> ReadResult {
        assert!(buffer.len() >= count);
        
        let read = self.cached_read_pos;
        let write = self.write_pos.load(Ordering::Acquire);
        let available = write - read;
        
        let to_read = available.min(count);
        
        if to_read > 0 {
            let read_idx = read & self.mask;
            let first_part = (self.capacity - read_idx).min(to_read);
            
            unsafe {
                // Read first part
                ptr::copy_nonoverlapping(
                    self.buffer.add(read_idx),
                    buffer.as_mut_ptr(),
                    first_part
                );
                
                // Read wrapped part
                if first_part < to_read {
                    ptr::copy_nonoverlapping(
                        self.buffer,
                        buffer.as_mut_ptr().add(first_part),
                        to_read - first_part
                    );
                }
            }
            
            // Update read position
            self.cached_read_pos = read + to_read;
            self.read_pos.store(self.cached_read_pos, Ordering::Release);
        }
        
        // Handle underflow by padding with zeros
        if to_read < count {
            let padding_start = to_read;
            let padding_count = count - to_read;
            buffer[padding_start..padding_start + padding_count].fill(0);
            
            self.stats.underflow_events.fetch_add(1, Ordering::Relaxed);
            
            ReadResult::Underflow {
                samples_read: to_read,
                samples_padded: padding_count,
            }
        } else {
            ReadResult::Success {
                samples_read: to_read,
            }
        }
    }
    
    /// Peek at available samples without consuming
    pub fn peek(&self, buffer: &mut [i16]) -> usize {
        let read = self.read_pos.load(Ordering::Acquire);
        let write = self.write_pos.load(Ordering::Acquire);
        let available = write - read;
        
        let to_peek = available.min(buffer.len());
        
        if to_peek > 0 {
            let read_idx = read & self.mask;
            let first_part = (self.capacity - read_idx).min(to_peek);
            
            unsafe {
                ptr::copy_nonoverlapping(
                    self.buffer.add(read_idx),
                    buffer.as_mut_ptr(),
                    first_part
                );
                
                if first_part < to_peek {
                    ptr::copy_nonoverlapping(
                        self.buffer,
                        buffer.as_mut_ptr().add(first_part),
                        to_peek - first_part
                    );
                }
            }
        }
        
        to_peek
    }
}
```

## Memory Ordering Strategy

### Ordering Requirements

| Operation | Producer | Consumer | Ordering | Rationale |
|-----------|----------|----------|----------|-----------|
| Write samples | write_pos.store | - | Release | Ensures samples visible before position update |
| Read samples | - | read_pos.store | Release | Ensures consumption visible to producer |
| Check space | - | read_pos.load | Acquire | Synchronizes with consumer's Release |
| Check available | write_pos.load | - | Acquire | Synchronizes with producer's Release |

### Synchronization Guarantees

```rust
// Producer synchronization point
impl AudioRingBuffer {
    fn producer_sync_point(&self) {
        // 1. Write samples to buffer (regular memory)
        // 2. Release fence ensures writes complete
        std::sync::atomic::fence(Ordering::Release);
        // 3. Update write_pos with Release ordering
        self.write_pos.store(new_pos, Ordering::Release);
        // Consumer will see all samples when it loads write_pos with Acquire
    }
}

// Consumer synchronization point  
impl AudioRingBuffer {
    fn consumer_sync_point(&self) {
        // 1. Load write_pos with Acquire ordering
        let write = self.write_pos.load(Ordering::Acquire);
        // 2. Acquire ensures we see all samples written before write_pos update
        // 3. Read samples from buffer (guaranteed visible)
        // 4. Update read_pos with Release ordering
        self.read_pos.store(new_pos, Ordering::Release);
    }
}
```

## Integration with Existing System

### Migration Path

```rust
// Phase 2.1: Adapter for existing AudioFrame consumers
pub struct RingBufferAdapter {
    ring: Arc<AudioRingBuffer>,
    metadata: Arc<MetadataRingBuffer>,
    frame_buffer: Vec<i16>,
}

impl RingBufferAdapter {
    /// Compatibility method for existing frame-based consumers
    pub fn recv_frame(&mut self) -> Option<AudioFrame> {
        // Read next frame boundary from metadata
        let boundary = self.metadata.read_next()?;
        
        // Read samples from ring buffer
        self.frame_buffer.resize(boundary.sample_count, 0);
        let result = self.ring.read_samples(
            boundary.sample_count,
            &mut self.frame_buffer
        );
        
        Some(AudioFrame {
            samples: self.frame_buffer.clone(),
            timestamp: boundary.timestamp,
            sample_rate: boundary.sample_rate,
            channels: 1,
            seq_no: boundary.seq_no,
        })
    }
}
```

### Producer Integration

```rust
// In audio/capture.rs
impl MicCapture {
    fn audio_callback(&mut self, data: &[i16]) {
        // Direct write to ring buffer (no allocation)
        match self.ring_buffer.write_samples(data) {
            Ok(()) => {
                // Success path - samples written
            }
            Err(BufferError::Overflow) => {
                // Log with rate limiting
                self.log_overflow_event();
            }
        }
        
        // Write frame boundary to metadata ring
        self.metadata_ring.write_boundary(FrameBoundary {
            start_idx: self.current_idx,
            sample_count: data.len(),
            timestamp: Instant::now(),
            seq_no: self.next_seq_no,
            sample_rate: self.config.sample_rate,
        });
        
        self.current_idx += data.len();
        self.next_seq_no += 1;
    }
}
```

## Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingBufferConfig {
    /// Buffer capacity in samples (will be rounded to power of 2)
    pub capacity_samples: usize,
    
    /// Overflow handling policy
    pub overflow_policy: OverflowPolicy,
    
    /// Enable underflow padding (vs blocking)
    pub underflow_pad: bool,
    
    /// Cache line size for alignment (default: 64)
    pub cache_line_size: usize,
    
    /// Enable debug assertions
    pub debug_mode: bool,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            capacity_samples: 32768,  // ~2 seconds at 16kHz
            overflow_policy: OverflowPolicy::DropNewest,
            underflow_pad: true,
            cache_line_size: 64,
            debug_mode: cfg!(debug_assertions),
        }
    }
}
```

## Telemetry & Monitoring

```rust
impl AudioRingBuffer {
    /// Get current buffer utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f32 {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Relaxed);
        let used = write - read;
        used as f32 / self.capacity as f32
    }
    
    /// Get comprehensive statistics
    pub fn stats(&self) -> BufferStatsSnapshot {
        BufferStatsSnapshot {
            samples_written: self.stats.samples_written.load(Ordering::Relaxed),
            samples_dropped: self.stats.samples_dropped.load(Ordering::Relaxed),
            overflow_events: self.stats.overflow_events.load(Ordering::Relaxed),
            underflow_events: self.stats.underflow_events.load(Ordering::Relaxed),
            current_utilization: self.utilization(),
            max_utilization: self.stats.max_utilization.load(Ordering::Relaxed) as f32 
                / self.capacity as f32,
        }
    }
    
    /// Reset statistics
    pub fn reset_stats(&self) {
        self.stats.samples_written.store(0, Ordering::Relaxed);
        self.stats.samples_dropped.store(0, Ordering::Relaxed);
        self.stats.overflow_events.store(0, Ordering::Relaxed);
        self.stats.underflow_events.store(0, Ordering::Relaxed);
        self.stats.max_utilization.store(0, Ordering::Relaxed);
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_power_of_two_sizing() {
        let rb = AudioRingBuffer::new(1000, OverflowPolicy::DropNewest);
        assert_eq!(rb.capacity, 1024);
        assert_eq!(rb.mask, 1023);
    }
    
    #[test]
    fn test_wrap_around() {
        let mut rb = AudioRingBuffer::new(16, OverflowPolicy::DropNewest);
        
        // Fill buffer to near capacity
        let data = vec![1i16; 14];
        rb.write_samples(&data).unwrap();
        
        // Write across boundary
        let wrap_data = vec![2i16; 8];
        rb.write_samples(&wrap_data).unwrap();
        
        // Read and verify wrap
        let mut buffer = vec![0i16; 22];
        rb.read_samples(22, &mut buffer);
        
        assert_eq!(&buffer[0..14], &vec![1i16; 14][..]);
        assert_eq!(&buffer[14..22], &vec![2i16; 8][..]);
    }
    
    #[test]
    fn test_concurrent_access() {
        use std::thread;
        use std::sync::Arc;
        
        let rb = Arc::new(AudioRingBuffer::new(8192, OverflowPolicy::DropNewest));
        
        let producer_rb = Arc::clone(&rb);
        let producer = thread::spawn(move || {
            for i in 0..1000 {
                let samples = vec![i as i16; 320];
                producer_rb.write_samples(&samples).ok();
                thread::sleep(Duration::from_micros(100));
            }
        });
        
        let consumer_rb = Arc::clone(&rb);
        let consumer = thread::spawn(move || {
            let mut buffer = vec![0i16; 320];
            let mut total_read = 0;
            
            while total_read < 320000 {
                let result = consumer_rb.read_samples(320, &mut buffer);
                if let ReadResult::Success { samples_read } = result {
                    total_read += samples_read;
                }
                thread::sleep(Duration::from_micros(100));
            }
            
            total_read
        });
        
        producer.join().unwrap();
        let total = consumer.join().unwrap();
        assert_eq!(total, 320000);
    }
}
```

### Stress Testing

```rust
#[test]
fn stress_test_high_throughput() {
    let rb = AudioRingBuffer::new(65536, OverflowPolicy::DropOldest);
    let start = Instant::now();
    let mut samples_processed = 0u64;
    
    // Simulate 60 seconds of audio at 48kHz
    while start.elapsed() < Duration::from_secs(60) {
        // Producer: 10ms frames
        let frame = vec![0i16; 480];
        rb.write_samples(&frame).unwrap();
        
        // Consumer: 20ms frames
        let mut buffer = vec![0i16; 960];
        rb.read_samples(960, &mut buffer);
        
        samples_processed += 960;
    }
    
    let stats = rb.stats();
    println!("Processed {} samples", samples_processed);
    println!("Dropped: {}", stats.samples_dropped);
    println!("Max utilization: {:.2}%", stats.max_utilization * 100.0);
}
```

### Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_write(c: &mut Criterion) {
    let mut rb = AudioRingBuffer::new(8192, OverflowPolicy::DropNewest);
    let samples = vec![0i16; 320];
    
    c.bench_function("write_320_samples", |b| {
        b.iter(|| {
            rb.write_samples(black_box(&samples))
        });
    });
}

fn benchmark_read(c: &mut Criterion) {
    let mut rb = AudioRingBuffer::new(8192, OverflowPolicy::DropNewest);
    let samples = vec![0i16; 8192];
    rb.write_samples(&samples).unwrap();
    
    let mut buffer = vec![0i16; 320];
    
    c.bench_function("read_320_samples", |b| {
        b.iter(|| {
            rb.read_samples(320, black_box(&mut buffer))
        });
    });
}

criterion_group!(benches, benchmark_write, benchmark_read);
criterion_main!(benches);
```

## Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Write latency (320 samples) | < 1μs | Audio callback deadline |
| Read latency (320 samples) | < 1μs | Processing thread deadline |
| Memory overhead | < 1MB | Embedded system constraint |
| Cache misses per operation | < 2 | Cache efficiency |
| CPU usage (48kHz stereo) | < 1% | Resource efficiency |

## Safety & Correctness

### Invariants

1. **Capacity is always power of 2**: Enforced in constructor
2. **Write position >= Read position**: Monotonic advancement
3. **No data races**: SPSC with proper memory ordering
4. **No ABA problem**: Positions only increment
5. **Memory safety**: Unsafe blocks are sound

### Debug Assertions

```rust
#[cfg(debug_assertions)]
impl AudioRingBuffer {
    fn debug_check_invariants(&self) {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Relaxed);
        
        debug_assert!(write >= read, "Write position behind read!");
        debug_assert!(write - read <= self.capacity, "Buffer overflow!");
        debug_assert!(self.capacity.is_power_of_two(), "Capacity not power of 2!");
        debug_assert_eq!(self.mask, self.capacity - 1, "Mask incorrect!");
    }
}
```

## Implementation Timeline

### Week 1: Core Implementation
- [ ] Basic ring buffer structure
- [ ] Write/read operations
- [ ] Unit tests

### Week 2: Integration
- [ ] Replace crossbeam channel
- [ ] Adapter for existing code
- [ ] Integration tests

### Week 3: Optimization & Testing
- [ ] Performance benchmarks
- [ ] Stress testing
- [ ] Telemetry integration
- [ ] Documentation

## Conclusion

This lock-free ring buffer design provides:

1. **Real-time guarantees**: Wait-free operations in audio callback
2. **Memory efficiency**: Zero allocations, cache-friendly layout
3. **Correctness**: Proper synchronization with formal memory ordering
4. **Observability**: Comprehensive metrics and debugging support
5. **Compatibility**: Smooth migration path from current implementation

The design prioritizes simplicity and correctness while achieving the performance requirements for production real-time audio processing.