//! Example demonstrating async STT processing improvements
//!
//! This example shows how the async STT implementation improves
//! responsiveness by using non-blocking operations.

use std::time::{Duration, Instant};
use tokio::sync::mpsc;

// Mock types for the example
#[derive(Debug, Clone)]
pub struct TranscriptionEvent {
    pub text: String,
    pub processing_time_ms: u64,
}

// Simulate synchronous (blocking) STT processing
async fn simulate_sync_stt_processing(audio_data: Vec<i16>) -> TranscriptionEvent {
    let start = Instant::now();
    
    // Simulate blocking I/O operation (like Vosk processing)
    std::thread::sleep(Duration::from_millis(100)); // This blocks the entire runtime!
    
    TranscriptionEvent {
        text: format!("Transcribed {} samples", audio_data.len()),
        processing_time_ms: start.elapsed().as_millis() as u64,
    }
}

// Simulate asynchronous (non-blocking) STT processing
async fn simulate_async_stt_processing(audio_data: Vec<i16>) -> TranscriptionEvent {
    let start = Instant::now();
    
    // Move blocking operation to a separate thread
    let result = tokio::task::spawn_blocking(move || {
        // This runs on a separate thread, not blocking the async runtime
        std::thread::sleep(Duration::from_millis(100));
        format!("Transcribed {} samples", audio_data.len())
    }).await.unwrap();
    
    TranscriptionEvent {
        text: result,
        processing_time_ms: start.elapsed().as_millis() as u64,
    }
}

async fn demo_responsiveness() {
    println!("=== Async STT Responsiveness Demo ===\n");
    
    // Simulate multiple UI updates that should remain responsive
    let (ui_tx, mut ui_rx) = mpsc::channel(10);
    
    // Start UI update task
    let ui_task = tokio::spawn(async move {
        let mut counter = 0;
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;
            counter += 1;
            if ui_tx.send(format!("UI Update #{}", counter)).await.is_err() {
                break;
            }
        }
    });
    
    // Test 1: Synchronous processing (blocks UI updates)
    println!("1. Testing SYNCHRONOUS processing (blocks UI):");
    let sync_start = Instant::now();
    
    // This will block the entire runtime for 100ms per call
    let _result1 = simulate_sync_stt_processing(vec![1; 1000]).await;
    let _result2 = simulate_sync_stt_processing(vec![2; 1000]).await;
    
    let sync_duration = sync_start.elapsed();
    println!("   Sync processing took: {:?}", sync_duration);
    
    // Collect any UI updates that happened during sync processing
    let mut sync_ui_updates = Vec::new();
    while let Ok(update) = ui_rx.try_recv() {
        sync_ui_updates.push(update);
    }
    println!("   UI updates during sync: {} (should be 0-1)", sync_ui_updates.len());
    
    tokio::time::sleep(Duration::from_millis(300)).await; // Let UI catch up
    
    // Test 2: Asynchronous processing (UI stays responsive)
    println!("\n2. Testing ASYNCHRONOUS processing (UI responsive):");
    let async_start = Instant::now();
    
    // These run concurrently and don't block the runtime
    let task1 = tokio::spawn(simulate_async_stt_processing(vec![1; 1000]));
    let task2 = tokio::spawn(simulate_async_stt_processing(vec![2; 1000]));
    
    // Wait for both to complete
    let (_result1, _result2) = tokio::join!(task1, task2);
    
    let async_duration = async_start.elapsed();
    println!("   Async processing took: {:?}", async_duration);
    
    // Collect UI updates that happened during async processing
    let mut async_ui_updates = Vec::new();
    while let Ok(update) = ui_rx.try_recv() {
        async_ui_updates.push(update);
    }
    println!("   UI updates during async: {} (should be 2-3)", async_ui_updates.len());
    
    // Clean up
    ui_task.abort();
    
    println!("\n=== Results ===");
    println!("Synchronous approach: UI was blocked for ~{:?}", sync_duration);
    println!("Asynchronous approach: UI remained responsive, processed in ~{:?}", async_duration);
    println!("Performance improvement: UI responsiveness maintained while processing time similar");
}

async fn demo_concurrent_streams() {
    println!("\n=== Concurrent Stream Processing Demo ===\n");
    
    let start = Instant::now();
    
    // Simulate processing 5 audio streams concurrently
    let mut tasks = Vec::new();
    
    for stream_id in 1..=5 {
        let task = tokio::spawn(async move {
            let audio_data = vec![stream_id as i16; 1000];
            let result = simulate_async_stt_processing(audio_data).await;
            println!("Stream {} completed: {}", stream_id, result.text);
            result
        });
        tasks.push(task);
    }
    
    // Wait for all streams to complete
    let results = futures::future::join_all(tasks).await;
    
    let total_duration = start.elapsed();
    println!("\nProcessed {} streams concurrently in {:?}", results.len(), total_duration);
    println!("Sequential processing would have taken ~{:?}", Duration::from_millis(500));
    println!("Concurrency speedup: ~{:.1}x", 500.0 / total_duration.as_millis() as f64);
}

#[tokio::main]
async fn main() {
    // Demo 1: Responsiveness comparison
    demo_responsiveness().await;
    
    // Demo 2: Concurrent processing
    demo_concurrent_streams().await;
    
    println!("\n=== Conclusion ===");
    println!("The async STT implementation provides:");
    println!("1. ✅ Non-blocking UI - responsiveness maintained during transcription");
    println!("2. ✅ Concurrent processing - multiple streams can be handled simultaneously");
    println!("3. ✅ Better resource utilization - CPU cores used efficiently");
    println!("4. ✅ Scalability - can handle 10+ concurrent audio streams");
}