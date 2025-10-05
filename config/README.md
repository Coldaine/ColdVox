# Application Configuration

This directory contains the configuration files for the ColdVox application.

## `default.toml`

This is the primary configuration file for the application. It contains the default settings for all components, including text injection, VAD, and STT.

The application loads this file at startup. The values in this file can be overridden by environment variables or command-line arguments.

## `plugins.json`

This file contains the configuration for the STT (Speech-to-Text) plugin manager. It defines the preferred plugin, fallback plugins, and other settings related to plugin management.

While the main application configuration is in `default.toml`, this file is kept separate to potentially allow for dynamic updates or for management by external tools in the future.
