# Vendored Vosk Library

This directory contains a vendored copy of the Vosk speech recognition library for Linux x86_64.

## Contents

- `vosk-linux-x86_64-0.3.45.zip` - Official Vosk binary release
- `LICENSE` - Apache License 2.0
- `NOTICE` - Copyright and attribution notice

## Version

- **Version**: 0.3.45
- **Platform**: Linux x86_64
- **Source**: https://github.com/alphacep/vosk-api/releases/tag/v0.3.45

## Why Vendored?

This library is vendored to:
1. Ensure consistent CI builds
2. Avoid download failures during GitHub Actions runs
3. Lock to a known-working version

## Updating

To update to a new version:
1. Download from https://github.com/alphacep/vosk-api/releases
2. Replace the zip file
3. Update version references in workflows and this README
4. Test thoroughly before committing

## License Compliance

Vosk is distributed under the Apache License 2.0. We include proper attribution
and the license text as required. See LICENSE and NOTICE files in this directory.
