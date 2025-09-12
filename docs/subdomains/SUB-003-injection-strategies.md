---
id: SUB-003
title: Injection Strategies Subdomain
level: subdomain
status: drafting
owners:
  - CDIS
criticality: 4
parent: DOM-003
pillar_trace:
  - PIL-003
  - DOM-003
  - SUB-003
---

# Injection Strategies Subdomain [SUB-003]

The Injection Strategies Subdomain is concerned with the specific methods and backends used to perform text injection. It covers the variety of techniques available, from simple clipboard operations to complex accessibility API interactions.

Key capabilities include:
- **Clipboard Backend**: A universal method that copies text to the system clipboard and simulates a paste command.
- **Accessibility API Backend (AT-SPI)**: A more direct and reliable method for Linux that uses accessibility interfaces to insert text directly into applications.
- **Keyboard Emulation Backends (YDotool, Enigo)**: Methods that simulate keyboard input, character by character. These serve as powerful fallbacks.
- **Combined Strategies**: "Combo" methods that chain multiple techniques for increased reliability, such as copying to the clipboard and then using an accessibility API to paste.
- **Method Fallback Chains**: A prioritized list of backends to try in sequence, ensuring that if one method fails, another can be attempted.
