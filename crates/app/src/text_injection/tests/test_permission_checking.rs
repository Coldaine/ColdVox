#[cfg(test)]
mod tests {
    #[cfg(feature = "text-injection-ydotool")]
    use crate::text_injection::ydotool_injector::YdotoolInjector;
    use std::process::Command;

    #[test]
    fn test_binary_existence_check() {
        // Test with a binary that should exist
        let output = Command::new("which")
            .arg("ls")
            .output();
        
        assert!(output.is_ok());
        assert!(output.unwrap().status.success());
        
        // Test with a binary that shouldn't exist
        let output = Command::new("which")
            .arg("nonexistent_binary_xyz123")
            .output();
        
        assert!(output.is_ok());
        assert!(!output.unwrap().status.success());
    }
    
    #[cfg(feature = "text-injection-ydotool")]
    #[test]
    fn test_ydotool_availability() {
        let config = InjectionConfig::default();
        let injector = YdotoolInjector::new(config);
        let _available = injector.is_available();
    }
    
    #[test]
    fn test_permission_mode_check() {
        use std::os::unix::fs::PermissionsExt;
        
        // Check /usr/bin/ls or similar common executable
        if let Ok(metadata) = std::fs::metadata("/usr/bin/ls") {
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            
            // Should have at least execute permission for owner
            assert!(mode & 0o100 != 0);
        }
    }
}