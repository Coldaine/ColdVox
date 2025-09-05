//! Tests for async STT processing functionality

use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Test that async operations don't block the runtime
#[tokio::test]
async fn test_async_non_blocking() {
        // Create a task that simulates blocking work in spawn_blocking
        let blocking_task = tokio::spawn(async {
            tokio::task::spawn_blocking(|| {
                // Simulate heavy STT processing
                std::thread::sleep(Duration::from_millis(100));
                "transcription_result"
            }).await.unwrap()
        });

        // Create a concurrent task that should complete quickly
        let quick_task = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            "quick_result"
        });

        let start = Instant::now();
        
        // Both tasks should run concurrently
        let (blocking_result, quick_result) = tokio::join!(blocking_task, quick_task);
        
        let elapsed = start.elapsed();
        
        // Verify both completed successfully
        assert_eq!(blocking_result.unwrap(), "transcription_result");
        assert_eq!(quick_result.unwrap(), "quick_result");
        
        // The total time should be close to the blocking operation time,
        // not the sum of both (which would indicate sequential execution)
        assert!(elapsed < Duration::from_millis(120), "Should complete concurrently, took {:?}", elapsed);
        assert!(elapsed >= Duration::from_millis(100), "Should take at least as long as blocking operation, took {:?}", elapsed);
    }

    /// Test concurrent processing of multiple streams
    #[tokio::test]
    async fn test_concurrent_stream_processing() {
        let start = Instant::now();
        
        // Simulate 3 concurrent STT streams
        let mut tasks = Vec::new();
        
        for i in 1..=3 {
            let task = tokio::spawn(async move {
                tokio::task::spawn_blocking(move || {
                    // Simulate per-stream processing time
                    std::thread::sleep(Duration::from_millis(50));
                    format!("stream_{}_result", i)
                }).await.unwrap()
            });
            tasks.push(task);
        }
        
        // Wait for all streams to complete
        let results = futures::future::join_all(tasks).await;
        
        let elapsed = start.elapsed();
        
        // Verify all streams completed
        assert_eq!(results.len(), 3);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.as_ref().unwrap(), &format!("stream_{}_result", i + 1));
        }
        
        // Concurrent processing should take roughly the time of one operation,
        // not the sum of all (which would be ~150ms for sequential)
        assert!(elapsed < Duration::from_millis(100), "Concurrent processing took too long: {:?}", elapsed);
        assert!(elapsed >= Duration::from_millis(50), "Should take at least as long as one operation: {:?}", elapsed);
    }

    /// Test channel communication doesn't block
    #[tokio::test]
    async fn test_async_channel_communication() {
        let (tx, mut rx) = mpsc::channel(10);
        
        // Spawn a task that sends events after processing
        let sender_task = tokio::spawn(async move {
            for i in 1..=3 {
                // Simulate async STT processing
                let result = tokio::task::spawn_blocking(move || {
                    std::thread::sleep(Duration::from_millis(30));
                    format!("transcription_{}", i)
                }).await.unwrap();
                
                // Send result
                tx.send(result).await.unwrap();
            }
        });
        
        let start = Instant::now();
        let mut received_events = Vec::new();
        
        // Receive events as they arrive
        while received_events.len() < 3 {
            if let Some(event) = rx.recv().await {
                received_events.push(event);
                println!("Received event at {:?}: {}", start.elapsed(), event);
            }
        }
        
        sender_task.await.unwrap();
        
        let total_time = start.elapsed();
        
        // Verify all events received
        assert_eq!(received_events.len(), 3);
        assert_eq!(received_events[0], "transcription_1");
        assert_eq!(received_events[1], "transcription_2");
        assert_eq!(received_events[2], "transcription_3");
        
        // Events should arrive roughly every 30ms, total around 90ms
        assert!(total_time >= Duration::from_millis(80), "Should take at least 80ms: {:?}", total_time);
        assert!(total_time < Duration::from_millis(120), "Should complete within 120ms: {:?}", total_time);
    }
}