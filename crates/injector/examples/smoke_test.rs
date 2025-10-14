use injector::{inject_text, InjectOptions};

#[tokio::main]
async fn main() {
    println!("Attempting to inject text via the portal...");
    let text_to_inject = "Hello, world from the injector crate!\n";
    let options = InjectOptions::default();

    match inject_text(text_to_inject, &options).await {
        Ok(report) => {
            println!("Injection successful!");
            println!("  Backend used: {:?}", report.backend);
            println!("  Characters sent: {}", report.chars_sent);
            println!("  Time elapsed: {:?}", report.elapsed);
        }
        Err(e) => {
            eprintln!("Injection failed: {}", e);
        }
    }
}