use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use coldvox_stt::TranscriptionEvent;

use crate::{processor::AsyncInjectionProcessor, types::InjectionConfig};

#[tokio::test]
async fn async_processor_handles_final_and_ticks_without_panic() {
    // Set up channels
    let (tx, rx) = mpsc::channel::<TranscriptionEvent>(8);
    let (sd_tx, sd_rx) = mpsc::channel::<()>(1);

    // Default config: injector will construct and fall back appropriately in headless env
    let config = InjectionConfig::default();

    // Create the async processor
    let proc = AsyncInjectionProcessor::new(config, rx, sd_rx, None).await;

    // Spawn the processor in a task
    let proc_handle = tokio::spawn(async move { proc.run().await });

    // Send a Final event
    tx.send(TranscriptionEvent::Final {
        utterance_id: 1,
        text: "hello world".to_string(),
        words: None,
    })
    .await
    .unwrap();

    // Wait briefly for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Close the transcription channel to signal no more events
    drop(tx);

    // Send shutdown signal
    sd_tx.send(()).await.unwrap();

    // Close shutdown channel to ensure signal is sent
    drop(sd_tx);

    // Wait for processor to exit with a longer timeout
    match timeout(Duration::from_secs(10), proc_handle).await {
        Ok(result) => {
            result.expect("Processor task should not panic");
        }
        Err(_) => {
            panic!("Processor did not shutdown within 10 seconds - potential hang detected");
        }
    }
}
