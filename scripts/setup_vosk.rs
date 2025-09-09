use std::path::Path;

fn main() {
    let model_path = Path::new("models/vosk-model-small-en-us-0.15");
    
    // Verify model structure
    let required_dirs = ["am", "conf", "graph", "ivector"];
    for dir in &required_dirs {
        let dir_path = model_path.join(dir);
        if !dir_path.exists() {
            eprintln!("ERROR: Missing required directory: {}", dir_path.display());
            std::process::exit(1);
        }
    }
    
    println!("✓ Vosk model setup verified");
}