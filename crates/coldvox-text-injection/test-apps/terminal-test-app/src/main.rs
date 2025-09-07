use std::fs::File;
use std::io::{self, Read, Write};
use std::process;

fn main() -> io::Result<()> {
    let pid = process::id();
    let output_path = format!("/tmp/coldvox_terminal_test_{}.txt", pid);

    // Create the file. If it already exists, it will be truncated.
    let mut output_file = File::create(&output_path)?;

    // Read all bytes from stdin until EOF.
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    // Write the buffer to the file.
    output_file.write_all(buffer.as_bytes())?;

    Ok(())
}
