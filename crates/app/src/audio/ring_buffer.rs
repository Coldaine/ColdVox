use rtrb::{Consumer, Producer, RingBuffer};
use tracing::warn;

/// Audio ring buffer using rtrb (real-time safe)
pub struct AudioRingBuffer {
    producer: Producer<i16>,
    consumer: Consumer<i16>,
}

impl AudioRingBuffer {
    /// Create a new ring buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let (producer, consumer) = RingBuffer::new(capacity);
        Self { producer, consumer }
    }
    
    /// Split into producer and consumer for separate threads
    pub fn split(self) -> (AudioProducer, AudioConsumer) {
        (
            AudioProducer { producer: self.producer },
            AudioConsumer { consumer: self.consumer },
        )
    }
}

/// Producer half of the ring buffer (for audio callback thread)
pub struct AudioProducer {
    producer: Producer<i16>,
}

impl AudioProducer {
    /// Write samples from audio callback (non-blocking)
    pub fn write(&mut self, samples: &[i16]) -> Result<usize, ()> {
        let mut chunk = match self.producer.write_chunk(samples.len()) {
            Ok(chunk) => chunk,
            Err(_) => {
                warn!("Ring buffer overflow: tried to write {} samples, buffer full", samples.len());
                return Err(());
            }
        };

        // Write may wrap; fill both slices
        let (first, second) = chunk.as_mut_slices();
        let split = first.len();
        if split > 0 {
            first.copy_from_slice(&samples[..split]);
        }
        if second.len() > 0 {
            second.copy_from_slice(&samples[split..]);
        }
        chunk.commit_all();
        Ok(samples.len())
    }
    
    /// Check available space
    pub fn slots(&self) -> usize {
        self.producer.slots()
    }
}

/// Consumer half of the ring buffer (for processing thread)
pub struct AudioConsumer {
    consumer: Consumer<i16>,
}

impl AudioConsumer {
    /// Read available samples (non-blocking)
    pub fn read(&mut self, buffer: &mut [i16]) -> usize {
        let chunk = match self.consumer.read_chunk(buffer.len()) {
            Ok(chunk) => chunk,
            Err(rtrb::chunks::ChunkError::TooFewSlots(available)) => {
                if available == 0 {
                    return 0;
                }
                self.consumer.read_chunk(available).unwrap()
            }
        };

        let len = chunk.len();
        let (first, second) = chunk.as_slices();
        let split = first.len();
        if split > 0 {
            buffer[..split].copy_from_slice(first);
        }
        if second.len() > 0 {
            buffer[split..split + second.len()].copy_from_slice(second);
        }
        chunk.commit_all();
        len
    }
    
    /// Check available samples to read
    pub fn slots(&self) -> usize {
        self.consumer.slots()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_write_read() {
        let rb = AudioRingBuffer::new(1024);
        let (mut producer, mut consumer) = rb.split();
        
        let samples = vec![1, 2, 3, 4, 5];
        assert_eq!(producer.write(&samples).unwrap(), 5);
        
        let mut buffer = vec![0i16; 10];
        let read = consumer.read(&mut buffer);
        
        assert_eq!(read, 5);
        assert_eq!(&buffer[..5], &[1, 2, 3, 4, 5]);
    }
    
    #[test]
    fn test_overflow() {
        let rb = AudioRingBuffer::new(16);
        let (mut producer, mut _consumer) = rb.split();
        
        let samples = vec![1i16; 20];
        assert!(producer.write(&samples).is_err());
        
        let samples = vec![1i16; 16];
        assert!(producer.write(&samples).is_ok());
        
        let samples = vec![2i16; 1];
        assert!(producer.write(&samples).is_err());
    }
}