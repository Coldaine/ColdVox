document.addEventListener('DOMContentLoaded', () => {
    // Disable animations for consistent screenshots in tests
    Chart.defaults.animation = false;

    // --- DATA MODEL ---
    // This is the authoritative data. In a real app, this would be fetched.
    // I've corrected the file paths based on my exploration of the codebase.
    const seedData = {
      "schema_version": 1,
      "ui_prefs": { "theme": "dark", "rows_per_page": 25 },
      "system_info": {
        "name": "ColdVox",
        "version": "alpha",
        "architecture": "Rust workspace: Audio → VAD (Silero) → STT (Vosk) → Text Injection",
        "target_wer": "—",
        "target_latency": 1,
        "latency_units": "s",
        "expected_total_tests": 28
      },
      "components": {
        "audio_pipeline": {
          "name": "Audio Capture & Processing",
          "description": "CPAL capture, ring buffer, chunking, resample, watchdog",
          "priority": "critical",
          "subcomponents": {
            "device_and_capture": {
              "name": "Device Discovery & Capture Thread",
              "files": ["crates/coldvox-audio/src/device.rs", "crates/coldvox-audio/src/capture.rs"],
              "functions": ["DeviceManager", "AudioCaptureThread::spawn", "get_devices"],
              "tests": [
                {"id": "cvx_001","name": "Device Enumeration","description": "List devices across PipeWire/ALSA","type": "unit","status": "pending","coverage_target": 95,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_002","name": "Capture Startup/Shutdown","description": "Start, stream, stop without deadlocks","type": "integration","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_003","name": "Watchdog Recovery","description": "Auto-recover after 5s no-data","type": "edge_case","status": "pending","coverage_target": 100,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            },
            "buffering_and_resample": {
              "name": "Ring Buffer, Chunking (512 @16kHz), Resampler",
              "files": ["crates/coldvox-audio/src/ring_buffer.rs","frame_reader.rs","chunker.rs","resampler.rs"],
              "functions": ["AudioRingBuffer","AudioChunker","StreamResampler"],
              "tests": [
                {"id": "cvx_004","name": "Chunk Size Consistency","description": "Ensure 512-sample frames at 16kHz","type": "unit","status": "pending","coverage_target": 95,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_005","name": "Resampler Modes","description": "Fast/Balanced/Quality correctness & timing","type": "performance","status": "pending","coverage_target": 85,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            }
          }
        },
        "vad_system": {
          "name": "Voice Activity Detection (Silero)",
          "description": "ONNX-based Silero V5 with configurable thresholds",
          "priority": "high",
          "subcomponents": {
            "silero_engine": {
              "name": "SileroEngine & VAD Events",
              "files": ["crates/coldvox-vad-silero/src/silero_wrapper.rs","crates/coldvox-vad/src/types.rs"],
              "functions": ["VadEngine","VadEvent","VadState"],
              "tests": [
                {"id": "cvx_006","name": "Speech Start/End Debounce","description": "Accurate boundaries at default thresholds","type": "unit","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_007","name": "Config Reload","description": "Apply new VadConfig without regressions","type": "integration","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            }
          }
        },
        "stt_system": {
          "name": "Speech-to-Text (Vosk)",
          "description": "Event-based transcriber with model autodiscovery",
          "priority": "high",
          "subcomponents": {
            "vosk_transcriber": {
              "name": "VoskTranscriber & Events",
              "files": ["crates/coldvox-stt-vosk/src/vosk_transcriber.rs","crates/coldvox-stt/src/types.rs"],
              "functions": ["EventBasedTranscriber","TranscriptionEvent::{Partial,Final}"],
              "tests": [
                {"id": "cvx_008","name": "Model Autodiscovery","description": "Find models under models/vosk-model-* or env override","type": "integration","status": "pending","coverage_target": 95,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_009","name": "Partial Results Stream","description": "Low-latency partials under streaming load","type": "performance","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_010","name": "Finalize Utterance","description": "Graceful finalize on stop","type": "unit","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            }
          }
        },
        "text_injection_orchestrator": {
          "name": "Text Injection Orchestrator",
          "description": "StrategyManager + backend selection (AT-SPI, Clipboard, Combo, YDotool, KDotool, Enigo) with focus detection & allow/block lists",
          "priority": "critical",
          "subcomponents": {
            "backend_strategy": {
              "name": "Strategy & Fallback Chains",
              "files": ["crates/coldvox-text-injection/src/manager.rs","session.rs","processor.rs"],
              "functions": ["StrategyManager","InjectionProcessor","SessionConfig"],
              "tests": [
                {"id": "cvx_011","name": "Preferred Backend Path","description": "Select AT-SPI when available; fallback chain on failure","type": "integration","status": "pending","coverage_target": 95,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_012","name": "Timeout & Cooldowns","description": "Per-method timeouts and initial cooldown observed","type": "performance","status": "pending","coverage_target": 85,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_013","name": "Allow/Block Lists","description": "Regex/substring modes route or block as configured","type": "unit","status": "pending","coverage_target": 100,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            },
            "focus_and_injection": {
              "name": "Focus Provider + Backend Ops",
              "files": ["crates/coldvox-text-injection/src/focus.rs","atspi_injector.rs","clipboard_paste_injector.rs","combo_clip_ydotool.rs","ydotool_injector.rs","kdotool_injector.rs","enigo_injector.rs"],
              "functions": ["FocusProvider","inject_text","is_available"],
              "tests": [
                {"id": "cvx_014","name": "Focus Detection Determinism","description": "Deterministic focus in tests via injected FocusProvider","type": "unit","status": "pending","coverage_target": 95,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_015","name": "Clipboard Restore","description": "Preserve & restore clipboard when enabled","type": "integration","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_016","name": "Wayland/X11 Paths","description": "YDotool/KDotool availability and routing","type": "edge_case","status": "pending","coverage_target": 85,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            }
          }
        },
        "app_cli_and_hotkeys": {
          "name": "App CLI + Hotkeys/TUI",
          "description": "Main binary, global hotkeys (KDE KGlobalAccel), TUI dashboard, mic_probe",
          "priority": "medium",
          "subcomponents": {
            "hotkeys_and_tui": {
              "name": "Hotkey System & TUI",
              "files": ["crates/app/src/hotkey/*.rs","src/bin/tui_dashboard.rs","src/bin/mic_probe.rs"],
              "functions": ["Push-to-Talk","TUI controls"],
              "tests": [
                {"id": "cvx_017","name": "Push-to-Talk Flow","description": "Hold hotkey → speak → release injects","type": "integration","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_018","name": "Hotkey Conflicts","description": "No collisions with desktop defaults","type": "edge_case","status": "pending","coverage_target": 85,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            }
          }
        },
        "foundation_telemetry_gui": {
          "name": "Foundation, Telemetry, GUI Bridge",
          "description": "StateManager, graceful shutdown, metrics, and QML bridge stubs/integration plan",
          "priority": "medium",
          "subcomponents": {
            "foundation_and_metrics": {
              "name": "State, Shutdown, Pipeline Metrics",
              "files": ["crates/coldvox-foundation/src/state.rs","shutdown.rs","crates/coldvox-telemetry/src/pipeline_metrics.rs"],
              "functions": ["AppState transitions","ShutdownHandler","PipelineMetrics"],
              "tests": [
                {"id": "cvx_019","name": "State Transitions","description": "Validated transitions via StateManager","type": "unit","status": "pending","coverage_target": 95,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
                {"id": "cvx_020","name": "Graceful Shutdown","description": "No orphan threads/logging corruption","type": "integration","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            },
            "gui_bridge": {
              "name": "Qt/QML Bridge Service Layer",
              "files": ["crates/coldvox-gui/docs/implementation-plan.md"],
              "functions": ["GuiService","ServiceRegistry","event subscriptions"],
              "tests": [
                {"id": "cvx_021","name": "Service Interface Wiring","description": "GuiService ↔ audio/stt/vad/injection adapters","type": "integration","status": "pending","coverage_target": 85,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
              ]
            }
          }
        }
      },
      "tests_remaining": [
        {"id": "cvx_022","name": "Partial→Final Consistency","description": "No word regressions when finalizing","type": "accuracy","status": "pending","coverage_target": 100,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
        {"id": "cvx_023","name": "Unknown Focus Fallback","description": "Inject on unknown focus when enabled","type": "edge_case","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
        {"id": "cvx_024","name": "Backend Availability Probe","description": "is_available health checks for each backend","type": "unit","status": "pending","coverage_target": 95,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
        {"id": "cvx_025","name": "Clipboard+Paste Combo","description": "Combo path with AT-SPI paste and ydotool fallback","type": "integration","status": "pending","coverage_target": 90,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
        {"id": "cvx_026","name": "KDE Window Activation Assist","description": "KDotool assist on X11 when enabled","type": "integration","status": "pending","coverage_target": 85,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
        {"id": "cvx_027","name": "Latency Gauge Calibration","description": "Gauge reflects <1s target, ~0.2s actual","type": "performance","status": "pending","coverage_target": 100,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]},
        {"id": "cvx_028","name": "Live/CI Test Gating","description": "Gate slow E2E via env (COLDVOX_SLOW_TESTS)","type": "integration","status": "pending","coverage_target": 85,"link": "","actual_latency": 0.2,"last_updated": "2025-08-18","repo": "","issue": "","pr": "","workflow_url": "","flaky": false,"history": [{"date":"2025-08-18","status":"pending"}]}
      ]
    };

    // --- APPLICATION STATE ---
    let state = {
        data: {},
        filters: {
            priority: 'all',
            status: 'all',
            type: 'all',
            search: ''
        },
        // other UI state can go here
    };
    const LOCAL_STORAGE_KEY = 'coldvox_dashboard_data';

    // --- DATA HELPERS ---
    function getAllTests(data) {
        const allTests = [];
        if (!data.components) return allTests;

        Object.values(data.components).forEach(component => {
            const componentPriority = component.priority;
            Object.values(component.subcomponents).forEach(sub => {
                // Add component priority to each test for easier filtering
                const testsWithPriority = sub.tests.map(t => ({...t, priority: componentPriority}));
                allTests.push(...testsWithPriority);
            });
        });
        allTests.push(...(data.tests_remaining || []));
        return allTests;
    }


    // --- STATE MANAGEMENT ---
    function saveData() {
        try {
            localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(state.data));
        } catch (e) {
            console.error("Error saving data to localStorage", e);
        }
    }

    function loadData() {
        try {
            const savedData = localStorage.getItem(LOCAL_STORAGE_KEY);
            if (savedData) {
                const parsedData = JSON.parse(savedData);
                // Simple migration check
                if (parsedData.schema_version === seedData.schema_version) {
                    state.data = parsedData;
                    console.log("Loaded data from localStorage.");
                    return;
                }
            }
        } catch (e) {
            console.error("Error loading data from localStorage", e);
        }
        // If no saved data or version mismatch, use seed and save
        console.log("Using seed data.");
        state.data = JSON.parse(JSON.stringify(seedData)); // Deep copy
        saveData();
    }

    function render() {
        console.log("Rendering application state...");
        // This function will be expanded in later steps to call all the
        // specific render functions for each view and component.
        updateLiveSummary();

        const activeTab = document.querySelector('.tab-button.active').dataset.tab;

        if (activeTab === 'architecture') {
            renderArchitectureView();
        } else if (activeTab === 'details') {
            renderDetailsView();
        } else if (activeTab === 'dashboard') {
            renderDashboardView();
        } else if (activeTab === 'analytics') {
            renderAnalyticsView();
        }
    }

    // --- ANALYTICS VIEW ---
    function renderAnalyticsView() {
        renderStatusPieChart();
        renderProgressBarChart();
        renderTrendLineChart();
        renderD3Heatmap();
        renderLatencyGauge();
    }

    function renderStatusPieChart() {
        const allTests = getAllTests(state.data);
        const statusCounts = allTests.reduce((acc, test) => {
            acc[test.status] = (acc[test.status] || 0) + 1;
            return acc;
        }, {});

        const ctx = document.getElementById('status-pie-chart').getContext('2d');
        if (chartInstances.pie) chartInstances.pie.destroy();
        chartInstances.pie = new Chart(ctx, {
            type: 'pie',
            data: {
                labels: Object.keys(statusCounts),
                datasets: [{
                    data: Object.values(statusCounts),
                    backgroundColor: Object.keys(statusCounts).map(s => `var(--status-${s})`)
                }]
            },
            options: {
                responsive: true,
                plugins: {
                    legend: { position: 'top' },
                    title: { display: true, text: 'Test Status Distribution' }
                }
            }
        });
    }

    function renderProgressBarChart() {
        const { components } = state.data;
        const labels = Object.values(components).map(c => c.name);
        const data = Object.values(components).map(c => {
            const allTests = Object.values(c.subcomponents).flatMap(sub => sub.tests);
            const passed = allTests.filter(t => t.status === 'passed').length;
            return allTests.length > 0 ? (passed / allTests.length) * 100 : 0;
        });

        const ctx = document.getElementById('progress-bar-chart').getContext('2d');
        if (chartInstances.bar) chartInstances.bar.destroy();
        chartInstances.bar = new Chart(ctx, {
            type: 'bar',
            data: {
                labels,
                datasets: [{
                    label: '% Tests Passed',
                    data,
                    backgroundColor: 'var(--accent-blue)'
                }]
            },
            options: {
                indexAxis: 'y',
                responsive: true,
                plugins: {
                    legend: { display: false },
                    title: { display: true, text: 'Progress by Component' }
                }
            }
        });
    }

    function renderTrendLineChart() {
        const allTests = getAllTests(state.data);
        const passedTestsByDate = allTests
            .filter(t => t.status === 'passed')
            .reduce((acc, test) => {
                const date = test.last_updated;
                if (date) {
                    acc[date] = (acc[date] || 0) + 1;
                }
                return acc;
            }, {});

        const sortedDates = Object.keys(passedTestsByDate).sort();
        const cumulativeData = sortedDates.reduce((acc, date) => {
            const prev = acc.length > 0 ? acc[acc.length - 1] : { y: 0 };
            acc.push({ x: date, y: prev.y + passedTestsByDate[date] });
            return acc;
        }, []);

        const ctx = document.getElementById('trend-line-chart').getContext('2d');
        if (chartInstances.line) chartInstances.line.destroy();
        chartInstances.line = new Chart(ctx, {
            type: 'line',
            data: {
                datasets: [{
                    label: 'Cumulative Passed Tests',
                    data: cumulativeData,
                    borderColor: 'var(--accent-green)',
                    tension: 0.1
                }]
            },
            options: {
                responsive: true,
                scales: {
                    x: {
                        type: 'time',
                        time: { unit: 'day' }
                    }
                },
                plugins: {
                    title: { display: true, text: 'Progress Over Time' }
                }
            }
        });
    }

    function renderD3Heatmap() {
        const container = document.getElementById('d3-heatmap');
        container.innerHTML = ''; // Clear previous render
        // D3 Heatmap implementation is complex and will be a simplified version here
        container.innerHTML = '<h4>Component vs. Test Type Heatmap (D3)</h4><p><i>Full D3 heatmap implementation is a complex task for this context. This is a placeholder.</i></p>';
    }

    function renderLatencyGauge() {
        const allTests = getAllTests(state.data);
        const latencies = allTests.map(t => t.actual_latency).filter(l => typeof l === 'number');
        const avgLatency = latencies.length > 0 ? (latencies.reduce((a, b) => a + b, 0) / latencies.length) : 0;
        const targetLatency = state.data.system_info.target_latency;

        const ctx = document.getElementById('latency-gauge-chart').getContext('2d');
        if (chartInstances.gauge) chartInstances.gauge.destroy();
        chartInstances.gauge = new Chart(ctx, {
            type: 'doughnut',
            data: {
                labels: ['Actual Latency', 'Remaining'],
                datasets: [{
                    data: [avgLatency, Math.max(0, targetLatency * 1.5 - avgLatency)], // Scale gauge to 1.5x target
                    backgroundColor: ['var(--accent-yellow)', 'var(--bg-dark)'],
                    circumference: 180,
                    rotation: 270
                }]
            },
            options: {
                responsive: true,
                cutout: '70%',
                plugins: {
                    legend: { display: false },
                    title: { display: true, text: `Latency: ${avgLatency.toFixed(2)}s / ${targetLatency}s Target` }
                }
            }
        });
    }

    // --- DASHBOARD VIEW ---
    let sortState = { key: 'id', asc: true };
    let currentPage = 1;
    const rowsPerPage = 25; // from ui_prefs, but hardcoded for now

    function renderDashboardView() {
        const container = document.getElementById('dashboard-table-container');
        let allTests = getAllTests(state.data);

        // Apply filters
        allTests = allTests.filter(t => {
            const searchMatch = state.filters.search ? (t.name.toLowerCase().includes(state.filters.search) || t.id.toLowerCase().includes(state.filters.search)) : true;
            const priorityMatch = state.filters.priority === 'all' || t.priority === state.filters.priority;
            const statusMatch = state.filters.status === 'all' || t.status === state.filters.status;
            const typeMatch = state.filters.type === 'all' || t.type === state.filters.type;
            return searchMatch && priorityMatch && statusMatch && typeMatch;
        });

        // Apply sorting
        allTests.sort((a, b) => {
            let valA = a[sortState.key];
            let valB = b[sortState.key];
            if (typeof valA === 'string') {
                return sortState.asc ? valA.localeCompare(valB) : valB.localeCompare(valA);
            }
            return sortState.asc ? valA - valB : valB - valA;
        });

        // Apply pagination
        const totalPages = Math.ceil(allTests.length / rowsPerPage);
        const paginatedTests = allTests.slice((currentPage - 1) * rowsPerPage, currentPage * rowsPerPage);

        container.innerHTML = `
            <table id="dashboard-table">
                <thead>
                    <tr>
                        <th data-sort="id">ID</th>
                        <th data-sort="name">Name</th>
                        <th data-sort="type">Type</th>
                        <th data-sort="status">Status</th>
                        <th data-sort="last_updated">Last Updated</th>
                    </tr>
                </thead>
                <tbody>
                    ${paginatedTests.map(test => `
                        <tr draggable="true" data-testid="${test.id}">
                            <td>${test.id}</td>
                            <td>${test.name}</td>
                            <td>${test.type}</td>
                            <td><span class="status-badge status-${test.status}">${test.status}</span></td>
                            <td>${test.last_updated}</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
            <div class="pagination">
                ${Array.from({ length: totalPages }, (_, i) => `<button class="page-btn ${i + 1 === currentPage ? 'active' : ''}" data-page="${i + 1}">${i + 1}</button>`).join('')}
            </div>
        `;

        addDashboardEventListeners();
    }

    function addDashboardEventListeners() {
        // Sorting
        document.querySelectorAll('#dashboard-table th').forEach(th => {
            th.addEventListener('click', () => {
                const key = th.dataset.sort;
                if (sortState.key === key) {
                    sortState.asc = !sortState.asc;
                } else {
                    sortState.key = key;
                    sortState.asc = true;
                }
                renderDashboardView();
            });
        });

        // Pagination
        document.querySelectorAll('.pagination .page-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                currentPage = parseInt(btn.dataset.page, 10);
                renderDashboardView();
            });
        });

        // Drag and drop
        let draggedItemId = null;
        document.querySelectorAll('#dashboard-table tbody tr').forEach(row => {
            row.addEventListener('dragstart', (e) => {
                draggedItemId = e.target.dataset.testid;
                e.dataTransfer.effectAllowed = 'move';
            });
        });

        // This is a simplified DnD. A real one would have drop zones.
        // For now, we'll just cycle status on drop.
        const table = document.getElementById('dashboard-table');
        table.addEventListener('dragover', (e) => e.preventDefault());
        table.addEventListener('drop', (e) => {
            e.preventDefault();
            if (draggedItemId) {
                const test = findTestById(draggedItemId);
                if (test) {
                    const statuses = ['pending', 'in_progress', 'passed', 'failed', 'blocked', 'flaky'];
                    const currentIndex = statuses.indexOf(test.status);
                    test.status = statuses[(currentIndex + 1) % statuses.length];
                    saveData();
                    render();
                }
                draggedItemId = null;
            }
        });
    }

    function setupKeyboardShortcuts() {
        let gPressed = false;
        window.addEventListener('keydown', (e) => {
            if (e.key === 'g') {
                gPressed = true;
                setTimeout(() => { gPressed = false; }, 2000); // Reset after 2s
            }

            if (gPressed) {
                const keyMap = {
                    'a': 'architecture',
                    'd': 'details',
                    't': 'dashboard', // t for table/dashboard
                    'n': 'analytics' // n for analytics
                };
                if (keyMap[e.key]) {
                    e.preventDefault();
                    const tabButton = document.querySelector(`.tab-button[data-tab="${keyMap[e.key]}"]`);
                    if(tabButton) tabButton.click();
                    gPressed = false;
                }
            }

            if (e.key === '/') {
                e.preventDefault();
                document.getElementById('global-search').focus();
            }
        });
    }

    // --- DETAILS VIEW ---
    const chartInstances = {}; // To keep track of Chart.js instances

    function renderDetailsView() {
        const container = document.getElementById('details-accordions');
        container.innerHTML = ''; // Clear previous content
        const { components } = state.data;

        for (const [key, component] of Object.entries(components)) {
            const accordion = document.createElement('div');
            accordion.className = 'accordion';

            const allTests = Object.values(component.subcomponents).flatMap(sub => sub.tests);
            const passedCount = allTests.filter(t => t.status === 'passed').length;
            const totalCount = allTests.length;

            const accordionHeader = document.createElement('div');
            accordionHeader.className = 'accordion-header';
            accordionHeader.innerHTML = `
                <span>${component.name}</span>
                <div style="display: flex; align-items: center; gap: 1rem;">
                    <span>${passedCount} / ${totalCount} tests passed</span>
                    <canvas id="chart-${key}" width="40" height="40"></canvas>
                </div>
            `;

            const accordionContent = document.createElement('div');
            accordionContent.className = 'accordion-content';
            accordionContent.innerHTML = createSubcomponentTables(component.subcomponents);

            accordion.appendChild(accordionHeader);
            accordion.appendChild(accordionContent);
            container.appendChild(accordion);

            accordionHeader.addEventListener('click', () => {
                accordionContent.classList.toggle('active');
                accordionContent.style.display = accordionContent.classList.contains('active') ? 'block' : 'none';
            });

            // Render doughnut chart
            if (chartInstances[`chart-${key}`]) {
                chartInstances[`chart-${key}`].destroy();
            }
            const ctx = document.getElementById(`chart-${key}`).getContext('2d');
            chartInstances[`chart-${key}`] = new Chart(ctx, {
                type: 'doughnut',
                data: {
                    datasets: [{
                        data: [passedCount, totalCount - passedCount],
                        backgroundColor: ['var(--status-passed)', 'var(--bg-dark)'],
                        borderWidth: 0
                    }]
                },
                options: {
                    responsive: false,
                    cutout: '70%',
                    plugins: {
                        legend: { display: false },
                        tooltip: { enabled: false }
                    }
                }
            });
        }
        addTableEventListeners();
    }

    function createSubcomponentTables(subcomponents) {
        let html = '';
        for (const [key, sub] of Object.entries(subcomponents)) {
            html += `
                <h4>${sub.name}</h4>
                <table class="details-table">
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Name</th>
                            <th>Status</th>
                            <th>Link</th>
                            <th>Notes</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${sub.tests.map(test => `
                            <tr data-testid="${test.id}">
                                <td>${test.id}</td>
                                <td>${test.name}</td>
                                <td class="editable-status">
                                    <select data-field="status" data-testid="${test.id}">
                                        ${['pending', 'in_progress', 'passed', 'failed', 'blocked', 'flaky'].map(s => `<option value="${s}" ${test.status === s ? 'selected' : ''}>${s}</option>`).join('')}
                                    </select>
                                </td>
                                <td class="editable-text" data-field="link" data-testid="${test.id}" contenteditable="true">${test.link || ''}</td>
                                <td class="editable-text" data-field="description" data-testid="${test.id}" contenteditable="true">${test.description}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            `;
        }
        return html;
    }

    function addTableEventListeners() {
        document.querySelectorAll('.details-table .editable-status select').forEach(select => {
            select.addEventListener('change', handleStatusChange);
        });
        document.querySelectorAll('.details-table .editable-text').forEach(cell => {
            cell.addEventListener('blur', handleTextChange);
        });
    }

    function findTestById(testId) {
        for (const component of Object.values(state.data.components)) {
            for (const sub of Object.values(component.subcomponents)) {
                const test = sub.tests.find(t => t.id === testId);
                if (test) return test;
            }
        }
        const test = state.data.tests_remaining.find(t => t.id === testId);
        return test;
    }

    function handleStatusChange(event) {
        const select = event.target;
        const testId = select.dataset.testid;
        const newStatus = select.value;
        const test = findTestById(testId);

        if (!test) return;

        // Rule: Can't set to passed if coverage target not met (simulated)
        const coverageMet = true; // In a real scenario, this would be a real check
        if (newStatus === 'passed' && !coverageMet) {
            alert(`Cannot mark as passed. Coverage target of ${test.coverage_target}% is not met.`);
            select.value = test.status; // Revert UI
            return;
        }

        // Rule: Require reason for failure/block
        if (newStatus === 'failed' || newStatus === 'blocked') {
            const reason = prompt(`Reason for marking test as ${newStatus}:`);
            if (reason) {
                test.description += `\n[${newStatus.toUpperCase()}] ${reason}`;
            } else {
                select.value = test.status; // Revert if user cancels prompt
                return;
            }
        }

        test.status = newStatus;
        test.last_updated = new Date().toISOString().split('T')[0];
        saveData();
        render(); // Re-render to update component progress charts etc.
    }

    function handleTextChange(event) {
        const cell = event.target;
        const testId = cell.dataset.testid;
        const field = cell.dataset.field;
        const test = findTestById(testId);

        if (test) {
            test[field] = cell.textContent;
            saveData();
        }
    }

    // --- ARCHITECTURE VIEW ---
    function renderArchitectureView() {
        renderMermaidPipeline();
        renderD3Hierarchy();
    }

    function renderMermaidPipeline() {
        const container = document.getElementById('mermaid-pipeline');
        const graphDefinition = `
graph TD
    A[Audio Capture] --> B{VAD};
    B --> C{STT (Vosk)};
    C --> D[Text Injection Orchestrator];
`;
        // Ensure the container is empty before rendering
        container.innerHTML = '';
        mermaid.render('mermaid-svg-1', graphDefinition, (svgCode) => {
            container.innerHTML = svgCode;
        });
    }

    function renderD3Hierarchy() {
        const container = document.getElementById('d3-hierarchy');
        container.innerHTML = ''; // Clear previous render

        const { components } = state.data;
        if (!components) return;

        const hierarchyData = {
            name: "ColdVox Workspace",
            children: Object.entries(components).map(([key, component]) => ({
                name: component.name,
                priority: component.priority,
                children: Object.entries(component.subcomponents).map(([subKey, sub]) => ({
                    name: sub.name,
                    files: sub.files,
                    functions: sub.functions,
                    children: sub.tests.map(test => ({
                        name: test.name,
                        id: test.id,
                        status: test.status
                    }))
                }))
            }))
        };

        const width = container.clientWidth;
        const height = 600;

        const root = d3.hierarchy(hierarchyData);
        const treeLayout = d3.tree().size([height, width - 200]);
        treeLayout(root);

        const svg = d3.create("svg")
            .attr("width", width)
            .attr("height", height);

        const g = svg.append("g")
            .attr("transform", "translate(100,0)");

        const link = g.selectAll(".link")
            .data(root.links())
            .enter().append("path")
            .attr("class", "link")
            .attr("d", d3.linkHorizontal()
                .x(d => d.y)
                .y(d => d.x))
            .style("fill", "none")
            .style("stroke", "#555");

        const node = g.selectAll(".node")
            .data(root.descendants())
            .enter().append("g")
            .attr("class", "node")
            .attr("transform", d => `translate(${d.y},${d.x})`);

        node.append("circle")
            .attr("r", 5)
            .style("fill", d => {
                if (d.data.priority) {
                    return `var(--priority-${d.data.priority})`;
                }
                if (d.data.status) {
                    return `var(--status-${d.data.status})`;
                }
                return '#ccc';
            });

        node.append("text")
            .attr("dy", "0.31em")
            .attr("x", d => d.children ? -8 : 8)
            .attr("text-anchor", d => d.children ? "end" : "start")
            .text(d => d.data.name)
            .style("fill", "var(--text)");

        // Basic Tooltip
        const tooltip = d3.select("body").append("div")
            .attr("class", "tooltip")
            .style("position", "absolute")
            .style("visibility", "hidden")
            .style("background", "var(--bg-lighter)")
            .style("border", "1px solid var(--border-color)")
            .style("padding", "8px")
            .style("border-radius", "4px");

        node.on("mouseover", (event, d) => {
            let content = `<strong>${d.data.name}</strong>`;
            if(d.data.files) content += `<br/>Files: ${d.data.files.join(', ')}`;
            if(d.data.functions) content += `<br/>Functions: ${d.data.functions.join(', ')}`;
            tooltip.html(content).style("visibility", "visible");
        })
        .on("mousemove", (event) => tooltip.style("top", (event.pageY-10)+"px").style("left",(event.pageX+10)+"px"))
        .on("mouseout", () => tooltip.style("visibility", "hidden"));

        // Zoom/Pan
        const zoom = d3.zoom().on("zoom", (event) => {
            g.attr("transform", event.transform);
        });
        svg.call(zoom);

        container.append(svg.node());
    }

    // --- UI & CONTROLS ---

    function setupHeaderControls() {
        const searchInput = document.getElementById('global-search');
        searchInput.addEventListener('input', debounce((e) => {
            state.filters.search = e.target.value.toLowerCase();
            updateURL();
            render();
        }, 300));

        const demoDataBtn = document.getElementById('demo-data-toggle');
        demoDataBtn.addEventListener('click', () => {
            const allTests = getAllTests(state.data);
            const statuses = ['pending', 'in_progress', 'passed', 'failed', 'blocked', 'flaky'];
            allTests.forEach(test => {
                test.status = statuses[Math.floor(Math.random() * statuses.length)];
            });
            saveData();
            render();
            alert("Demo data has been applied.");
        });
    }

    function setupFooterControls() {
        document.getElementById('export-json').addEventListener('click', exportJSON);
        document.getElementById('export-csv').addEventListener('click', exportCSV);
        document.getElementById('export-pdf').addEventListener('click', exportPDF);
        document.getElementById('import-json').addEventListener('click', () => {
            document.getElementById('import-file-input').click();
        });
        document.getElementById('import-file-input').addEventListener('change', importJSON);
    }

    function exportJSON() {
        const dataStr = JSON.stringify(state.data, null, 2);
        const blob = new Blob([dataStr], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'coldvox-coverage.json';
        a.click();
        URL.revokeObjectURL(url);
    }

    function exportCSV() {
        const allTests = getAllTests(state.data);
        const csv = Papa.unparse(allTests);
        const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'coldvox-coverage.csv';
        a.click();
        URL.revokeObjectURL(url);
    }

    function importJSON(event) {
        const file = event.target.files[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = (e) => {
            try {
                const importedData = JSON.parse(e.target.result);
                if (importedData.schema_version !== state.data.schema_version) {
                    alert("Error: Schema version mismatch.");
                    return;
                }
                state.data = importedData;
                saveData();
                render();
                alert("Data imported successfully.");
            } catch (err) {
                alert("Error parsing JSON file.");
                console.error(err);
            }
        };
        reader.readAsText(file);
    }

    function exportPDF() {
        const { jsPDF } = window.jspdf;
        const doc = new jsPDF();
        const allTests = getAllTests(state.data);

        // Title
        doc.text("ColdVox Coverage Dashboard", 14, 20);
        doc.setFontSize(12);
        doc.text(`Generated on: ${new Date().toLocaleDateString()}`, 14, 28);

        // Add Mermaid SVG
        const svgElement = document.querySelector('#mermaid-pipeline svg');
        if (svgElement) {
            const svgString = new XMLSerializer().serializeToString(svgElement);
            doc.addSvgAsImage(svgString, 14, 40, 180, 60);
        }

        // Add table of tests
        const tableColumn = ["ID", "Name", "Status", "Type", "Priority"];
        const tableRows = [];

        allTests.forEach(test => {
            const testData = [
                test.id,
                test.name,
                test.status,
                test.type,
                test.priority || 'N/A'
            ];
            tableRows.push(testData);
        });

        doc.autoTable(tableColumn, tableRows, { startY: 110 });
        doc.save('coldvox-coverage.pdf');
    }

    // --- ROUTING & DEEP LINKING ---
    function updateURL() {
        const activeTab = document.querySelector('.tab-button.active').dataset.tab;
        const params = new URLSearchParams();
        params.set('tab', activeTab);
        if (state.filters.status !== 'all') params.set('status', state.filters.status);
        if (state.filters.priority !== 'all') params.set('priority', state.filters.priority);
        if (state.filters.type !== 'all') params.set('type', state.filters.type);
        if (state.filters.search) params.set('search', state.filters.search);

        history.pushState(null, '', '#' + params.toString());
    }

    function applyURLState() {
        const params = new URLSearchParams(window.location.hash.substring(1));
        const tab = params.get('tab') || 'architecture';

        // Set active tab
        document.querySelector('#tabs .active').classList.remove('active');
        document.querySelector(`[data-tab="${tab}"]`).classList.add('active');
        document.querySelectorAll('.view').forEach(v => v.style.display = 'none');
        document.getElementById(`${tab}-view`).style.display = 'block';

        // Apply filters
        state.filters.status = params.get('status') || 'all';
        state.filters.priority = params.get('priority') || 'all';
        state.filters.type = params.get('type') || 'all';
        state.filters.search = params.get('search') || '';

        // Update UI elements
        document.getElementById('filter-status').value = state.filters.status;
        document.getElementById('filter-priority').value = state.filters.priority;
        document.getElementById('filter-type').value = state.filters.type;
        document.getElementById('global-search').value = state.filters.search;
    }

    // --- INITIALIZATION ---
    function init() {
        mermaid.initialize({ startOnLoad: false, theme: 'dark', securityLevel: 'strict' });
        console.log("ColdVox Dashboard Initializing...");
        loadData();

        setupTabNavigation();
        setupHeaderControls();
        setupFooterControls();
        setupFilters();
        setupKeyboardShortcuts();

        applyURLState(); // Apply state from URL after setup
        render();

        window.addEventListener('popstate', () => {
            applyURLState();
            render();
        });
    }


    function setupFilters() {
        const priorityFilter = document.getElementById('filter-priority');
        const statusFilter = document.getElementById('filter-status');
        const typeFilter = document.getElementById('filter-type');

        priorityFilter.addEventListener('change', (e) => {
            state.filters.priority = e.target.value;
            render();
        });
        statusFilter.addEventListener('change', (e) => {
            state.filters.status = e.target.value;
            render();
        });
        typeFilter.addEventListener('change', (e) => {
            state.filters.type = e.target.value;
            render();
        });
    }

    function updateLiveSummary() {
        const allTests = getAllTests(state.data);
        const passedTests = allTests.filter(t => t.status === 'passed').length;
        const totalTests = allTests.length;

        document.getElementById('summary-coverage').textContent = `${passedTests}/${totalTests}`;

        const latencies = allTests.map(t => t.actual_latency).filter(l => typeof l === 'number');
        const avgLatency = latencies.length > 0 ? (latencies.reduce((a, b) => a + b, 0) / latencies.length) : 0;

        document.getElementById('summary-latency').textContent = `${avgLatency.toFixed(2)}${state.data.system_info.latency_units}`;
    }

    // --- UTILITIES ---
    function debounce(func, delay) {
        let timeout;
        return function(...args) {
            const context = this;
            clearTimeout(timeout);
            timeout = setTimeout(() => func.apply(context, args), delay);
        };
    }

    // --- INITIALIZATION ---
    function init() {
        console.log("ColdVox Dashboard Initializing...");
        loadData();
        setupTabNavigation();
        setupHeaderControls();
        setupFilters();
        setupKeyboardShortcuts();
        render();
    }

    // --- NAVIGATION ---
    function setupTabNavigation() {
        const tabs = document.getElementById('tabs');
        const views = document.querySelectorAll('.view');

        tabs.addEventListener('click', (e) => {
            if (e.target.classList.contains('tab-button')) {
                const tabName = e.target.dataset.tab;

                // Update button active state
                tabs.querySelector('.active').classList.remove('active');
                e.target.classList.add('active');

                // Show the correct view
                views.forEach(view => {
                    view.style.display = view.id === `${tabName}-view` ? 'block' : 'none';
                    view.classList.toggle('active', view.id === `${tabName}-view`);
                });
            }
        });
    }

    // Run the app
    init();
});