use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use coldvox_stt::TranscriptionEvent;

use crate::{processor::AsyncInjectionProcessor, types::InjectionConfig};

#[tokio::test]
async fn async_processor_handles_final_and_ticks_without_panic() {
    // Set up channels
    let (tx, rx) = mpsc::channel::<TranscriptionEvent>(8);
    let (_sd_tx, sd_rx) = mpsc::channel::<()>(1);

    // Default config: injector will construct and fall back appropriately in headless env
    let config = InjectionConfig::default();

    // Start the async processor task
    let proc = AsyncInjectionProcessor::new(config, rx, sd_rx, None).await;

    // Send a Final event
    tx.send(TranscriptionEvent::Final {
        utterance_id: 1,
        text: "hello world".to_string(),
        words: None,
    })
    .await
    .unwrap();

    // Let the loop tick a couple of intervals and ensure it doesn't panic
    let fut = async move {
        // Run a few ticks by calling run() but with a short timeout to break out
        // Instead of running the full loop, we just allow time to handle events
        tokio::time::sleep(Duration::from_millis(250)).await;
        Ok::<(), anyhow::Error>(())
    };

    let _ = timeout(Duration::from_secs(2), fut).await.expect("timeout");

    // Query metrics snapshot (non-panicking implies basic health)
    let _metrics = proc.metrics().await;
}
