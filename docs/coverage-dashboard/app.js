const STORAGE_KEY = 'coldvox_dashboard_state';
const SAVED_VIEWS_KEY = 'coldvox_dashboard_views';
const SCHEMA_VERSION = 1;

const STATUS_OPTIONS = ['pending', 'in_progress', 'passed', 'failed', 'blocked', 'flaky'];
const PRIORITY_OPTIONS = ['critical', 'high', 'medium', 'low'];
const TYPE_COLORS = {
  unit: '#3b82f6',
  integration: '#22c55e',
  performance: '#f59e0b',
  edge_case: '#a855f7',
  accuracy: '#ef4444',
};

const SEED = {
  schema_version: 1,
  ui_prefs: { theme: 'dark', rows_per_page: 25 },
  system_info: {
    name: 'ColdVox',
    version: 'alpha',
    architecture: 'Rust workspace: Audio → VAD (Silero) → STT (Vosk) → Text Injection',
    target_wer: '—',
    target_latency: 1,
    latency_units: 's',
    expected_total_tests: 28,
  },
  components: null,
  tests_remaining: [],
};

SEED.components = {
  audio_pipeline: {
    name: 'Audio Capture & Processing',
    description: 'CPAL capture, ring buffer, chunking, resample, watchdog',
    priority: 'critical',
    subcomponents: {
      device_and_capture: {
        name: 'Device Discovery & Capture Thread',
        files: ['crates/coldvox-audio/src/device.rs', 'crates/coldvox-audio/src/capture.rs'],
        functions: ['DeviceManager', 'AudioCaptureThread::spawn', 'get_devices'],
        tests: [
          { id: 'cvx_001', name: 'Device Enumeration', description: 'List devices across PipeWire/ALSA', type: 'unit', status: 'pending', coverage_target: 95, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_002', name: 'Capture Startup/Shutdown', description: 'Start, stream, stop without deadlocks', type: 'integration', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_003', name: 'Watchdog Recovery', description: 'Auto-recover after 5s no-data', type: 'edge_case', status: 'pending', coverage_target: 100, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
      buffering_and_resample: {
        name: 'Ring Buffer, Chunking (512 @16kHz), Resampler',
        files: ['crates/coldvox-audio/src/ring_buffer.rs', 'frame_reader.rs', 'chunker.rs', 'resampler.rs'],
        functions: ['AudioRingBuffer', 'AudioChunker', 'StreamResampler'],
        tests: [
          { id: 'cvx_004', name: 'Chunk Size Consistency', description: 'Ensure 512-sample frames at 16kHz', type: 'unit', status: 'pending', coverage_target: 95, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_005', name: 'Resampler Modes', description: 'Fast/Balanced/Quality correctness & timing', type: 'performance', status: 'pending', coverage_target: 85, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
    },
  },
  vad_system: {
    name: 'Voice Activity Detection (Silero)',
    description: 'ONNX-based Silero V5 with configurable thresholds',
    priority: 'high',
    subcomponents: {
      silero_engine: {
        name: 'SileroEngine & VAD Events',
        files: ['crates/coldvox-vad-silero/src/silero_wrapper.rs', 'crates/coldvox-vad/src/types.rs'],
        functions: ['VadEngine', 'VadEvent', 'VadState'],
        tests: [
          { id: 'cvx_006', name: 'Speech Start/End Debounce', description: 'Accurate boundaries at default thresholds', type: 'unit', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_007', name: 'Config Reload', description: 'Apply new VadConfig without regressions', type: 'integration', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
    },
  },
  stt_system: {
    name: 'Speech-to-Text (Vosk)',
    description: 'Event-based transcriber with model autodiscovery',
    priority: 'high',
    subcomponents: {
      vosk_transcriber: {
        name: 'VoskTranscriber & Events',
        files: ['crates/coldvox-stt-vosk/src/vosk_transcriber.rs', 'crates/coldvox-stt/src/types.rs'],
        functions: ['EventBasedTranscriber', 'TranscriptionEvent::{Partial,Final}'],
        tests: [
          { id: 'cvx_008', name: 'Model Autodiscovery', description: 'Find models under models/vosk-model-* or env override', type: 'integration', status: 'pending', coverage_target: 95, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_009', name: 'Partial Results Stream', description: 'Low-latency partials under streaming load', type: 'performance', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_010', name: 'Finalize Utterance', description: 'Graceful finalize on stop', type: 'unit', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
    },
  },
  text_injection_orchestrator: {
    name: 'Text Injection Orchestrator',
    description: 'StrategyManager + backend selection (AT-SPI, Clipboard, Combo, YDotool, KDotool, Enigo) with focus detection & allow/block lists',
    priority: 'critical',
    subcomponents: {
      backend_strategy: {
        name: 'Strategy & Fallback Chains',
        files: ['crates/coldvox-text-injection/src/manager.rs', 'session.rs', 'processor.rs'],
        functions: ['StrategyManager', 'InjectionProcessor', 'SessionConfig'],
        tests: [
          { id: 'cvx_011', name: 'Preferred Backend Path', description: 'Select AT-SPI when available; fallback chain on failure', type: 'integration', status: 'pending', coverage_target: 95, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_012', name: 'Timeout & Cooldowns', description: 'Per-method timeouts and initial cooldown observed', type: 'performance', status: 'pending', coverage_target: 85, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_013', name: 'Allow/Block Lists', description: 'Regex/substring modes route or block as configured', type: 'unit', status: 'pending', coverage_target: 100, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
      focus_and_injection: {
        name: 'Focus Provider + Backend Ops',
        files: ['crates/coldvox-text-injection/src/focus/*.rs', 'atspi_injector.rs', 'clipboard_injector.rs', 'combo_clip_ydotool.rs', 'ydotool_injector.rs', 'kdotool_injector.rs', 'enigo_injector.rs'],
        functions: ['FocusProvider', 'inject_text', 'is_available'],
        tests: [
          { id: 'cvx_014', name: 'Focus Detection Determinism', description: 'Deterministic focus in tests via injected FocusProvider', type: 'unit', status: 'pending', coverage_target: 95, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_015', name: 'Clipboard Restore', description: 'Preserve & restore clipboard when enabled', type: 'integration', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_016', name: 'Wayland/X11 Paths', description: 'YDotool/KDotool availability and routing', type: 'edge_case', status: 'pending', coverage_target: 85, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
    },
  },
  app_cli_and_hotkeys: {
    name: 'App CLI + Hotkeys/TUI',
    description: 'Main binary, global hotkeys (KDE KGlobalAccel), TUI dashboard, mic_probe',
    priority: 'medium',
    subcomponents: {
      hotkeys_and_tui: {
        name: 'Hotkey System & TUI',
        files: ['crates/app/src/hotkey/*.rs', 'src/bin/tui_dashboard.rs', 'src/bin/mic_probe.rs'],
        functions: ['Push-to-Talk', 'TUI controls'],
        tests: [
          { id: 'cvx_017', name: 'Push-to-Talk Flow', description: 'Hold hotkey → speak → release injects', type: 'integration', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_018', name: 'Hotkey Conflicts', description: 'No collisions with desktop defaults', type: 'edge_case', status: 'pending', coverage_target: 85, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
    },
  },
  foundation_telemetry_gui: {
    name: 'Foundation, Telemetry, GUI Bridge',
    description: 'StateManager, graceful shutdown, metrics, and QML bridge stubs/integration plan',
    priority: 'medium',
    subcomponents: {
      foundation_and_metrics: {
        name: 'State, Shutdown, Pipeline Metrics',
        files: ['crates/coldvox-foundation/src/state.rs', 'shutdown.rs', 'crates/coldvox-telemetry/src/pipeline_metrics.rs'],
        functions: ['AppState transitions', 'ShutdownHandler', 'PipelineMetrics'],
        tests: [
          { id: 'cvx_019', name: 'State Transitions', description: 'Validated transitions via StateManager', type: 'unit', status: 'pending', coverage_target: 95, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
          { id: 'cvx_020', name: 'Graceful Shutdown', description: 'No orphan threads/logging corruption', type: 'integration', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
      gui_bridge: {
        name: 'Qt/QML Bridge Service Layer',
        files: ['crates/coldvox-gui/docs/implementation-plan.md'],
        functions: ['GuiService', 'ServiceRegistry', 'event subscriptions'],
        tests: [
          { id: 'cvx_021', name: 'Service Interface Wiring', description: 'GuiService ↔ audio/stt/vad/injection adapters', type: 'integration', status: 'pending', coverage_target: 85, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
        ],
      },
    },
  },
};

SEED.tests_remaining = [
  { id: 'cvx_022', name: 'Partial→Final Consistency', description: 'No word regressions when finalizing', type: 'accuracy', status: 'pending', coverage_target: 100, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
  { id: 'cvx_023', name: 'Unknown Focus Fallback', description: 'Inject on unknown focus when enabled', type: 'edge_case', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
  { id: 'cvx_024', name: 'Backend Availability Probe', description: 'is_available health checks for each backend', type: 'unit', status: 'pending', coverage_target: 95, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
  { id: 'cvx_025', name: 'Clipboard+Paste Combo', description: 'Combo path with AT-SPI paste and ydotool fallback', type: 'integration', status: 'pending', coverage_target: 90, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
  { id: 'cvx_026', name: 'KDE Window Activation Assist', description: 'KDotool assist on X11 when enabled', type: 'integration', status: 'pending', coverage_target: 85, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
  { id: 'cvx_027', name: 'Latency Gauge Calibration', description: 'Gauge reflects <1s target, ~0.2s actual', type: 'performance', status: 'pending', coverage_target: 100, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
  { id: 'cvx_028', name: 'Live/CI Test Gating', description: 'Gate slow E2E via env (COLDVOX_SLOW_TESTS)', type: 'integration', status: 'pending', coverage_target: 85, link: '', actual_latency: 0.2, last_updated: '2025-08-18', repo: '', issue: '', pr: '', workflow_url: '', flaky: false, history: [{ date: '2025-08-18', status: 'pending' }] },
];

const mermaidDefinition = `flowchart LR
  A[Audio Capture] --> B[VAD (Silero)]
  B --> C[STT (Vosk)]
  C --> D[Text Injection Orchestrator]
  D --> E[Target Application]
`;

function deepClone(obj) {
  return typeof structuredClone === 'function' ? structuredClone(obj) : JSON.parse(JSON.stringify(obj));
}

function getAllTestsFromData(data) {
  const tests = [];
  Object.entries(data.components).forEach(([componentKey, component]) => {
    Object.entries(component.subcomponents).forEach(([subKey, sub]) => {
      sub.tests.forEach((test) => {
        tests.push({
          ...test,
          componentKey,
          componentName: component.name,
          componentPriority: component.priority,
          subcomponentKey: subKey,
          subcomponentName: sub.name,
        });
      });
    });
  });
  data.tests_remaining.forEach((test) => {
    tests.push({
      ...test,
      componentKey: 'tests_remaining',
      componentName: 'Cross-Cutting Coverage',
      componentPriority: 'medium',
      subcomponentKey: 'tests_remaining',
      subcomponentName: 'Additional Coverage',
    });
  });
  return tests;
}

function migrateState(data) {
  const cloned = deepClone(data);
  if (!cloned.schema_version || cloned.schema_version < 1) {
    cloned.schema_version = 1;
  }
  for (const test of getAllTestsFromData(cloned)) {
    if (typeof test.actual_latency !== 'number') {
      test.actual_latency = Number(test.actual_latency) || 0;
    }
    if (!test.history || !Array.isArray(test.history)) {
      test.history = [{ date: test.last_updated || new Date().toISOString(), status: test.status }];
    }
    if (test.coverage_actual === undefined) {
      test.coverage_actual = test.coverage_target;
    }
  }
  return cloned;
}

function loadState() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      const initial = migrateState(SEED);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(initial));
      return initial;
    }
    const parsed = JSON.parse(raw);
    if (!parsed.schema_version || parsed.schema_version < SCHEMA_VERSION) {
      const migrated = migrateState(parsed);
      migrated.schema_version = SCHEMA_VERSION;
      localStorage.setItem(STORAGE_KEY, JSON.stringify(migrated));
      return migrated;
    }
    return migrateState(parsed);
  } catch (error) {
    console.error('Failed to load state, restoring seed', error);
    const initial = migrateState(SEED);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(initial));
    return initial;
  }
}

function saveState(state) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function loadSavedViews() {
  try {
    const raw = localStorage.getItem(SAVED_VIEWS_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch (error) {
    console.error('Unable to load saved views', error);
    return [];
  }
}

function saveViews(views) {
  localStorage.setItem(SAVED_VIEWS_KEY, JSON.stringify(views));
}

let appState = loadState();
let savedViews = loadSavedViews();
let filters = { priority: [], status: [], type: [] };
let pagination = { page: 1, perPage: appState.ui_prefs?.rows_per_page || 25 };
let sortState = { column: 'id', direction: 'asc' };
let currentTab = 'dashboard';

const charts = {};

function setTab(tab) {
  currentTab = tab;
  document.querySelectorAll('.tab-panel').forEach((panel) => {
    panel.hidden = panel.id !== tab;
  });
  document.querySelectorAll('.tab-button').forEach((button) => {
    const isActive = button.dataset.tab === tab;
    button.setAttribute('aria-selected', isActive);
  });
  render();
  updateHash();
}

function updateHash() {
  const params = new URLSearchParams();
  if (filters.priority.length) params.set('priority', filters.priority.join(','));
  if (filters.status.length) params.set('status', filters.status.join(','));
  if (filters.type.length) params.set('type', filters.type.join(','));
  params.set('tab', currentTab);
  window.location.hash = `${currentTab}?${params.toString()}`;
}

function applyHash() {
  if (!window.location.hash) return;
  const [tabPart, query] = window.location.hash.slice(1).split('?');
  if (tabPart && ['architecture', 'details', 'dashboard', 'analytics'].includes(tabPart)) {
    currentTab = tabPart;
  }
  const params = new URLSearchParams(query || '');
  filters.priority = params.get('priority')?.split(',').filter(Boolean) || [];
  filters.status = params.get('status')?.split(',').filter(Boolean) || [];
  filters.type = params.get('type')?.split(',').filter(Boolean) || [];
}

function sanitize(text) {
  const div = document.createElement('div');
  div.textContent = text ?? '';
  return div.innerHTML;
}

function formatDate(date) {
  const d = new Date(date);
  return Number.isNaN(d.valueOf()) ? '' : d.toISOString().split('T')[0];
}

function computeStats(state) {
  const tests = getAllTestsFromData(state);
  const total = tests.length;
  const completed = tests.filter((t) => t.status === 'passed').length;
  const avgLatency = tests.length ? tests.reduce((acc, t) => acc + Number(t.actual_latency || 0), 0) / tests.length : 0;
  const statusCounts = Object.fromEntries(STATUS_OPTIONS.map((status) => [status, 0]));
  tests.forEach((test) => {
    statusCounts[test.status] = (statusCounts[test.status] || 0) + 1;
  });
  const byComponent = {};
  tests.forEach((test) => {
    byComponent[test.componentName] = byComponent[test.componentName] || { total: 0, passed: 0, priority: test.componentPriority };
    byComponent[test.componentName].total += 1;
    if (test.status === 'passed') byComponent[test.componentName].passed += 1;
  });
  return { total, completed, avgLatency, statusCounts, byComponent };
}

function meetsCoverage(test) {
  return (test.coverage_actual ?? test.coverage_target) >= test.coverage_target;
}

function updateFlakiness(test) {
  if (!test.history || test.history.length < 3) {
    test.flaky = false;
    return;
  }
  let transitions = 0;
  for (let i = 1; i < test.history.length; i += 1) {
    if (test.history[i].status !== test.history[i - 1].status) transitions += 1;
  }
  test.flaky = transitions >= 2;
}

function updateSummary() {
  const stats = computeStats(appState);
  document.getElementById('summary-overall').textContent = `Overall Coverage: ${stats.completed}/${stats.total} tests passed`;
  document.getElementById('summary-latency').textContent = `Avg. Latency: ${stats.avgLatency.toFixed(2)}${appState.system_info.latency_units}`;
  document.getElementById('footer-progress').textContent = `Target Latency < ${appState.system_info.target_latency}${appState.system_info.latency_units} | Actual ${stats.avgLatency.toFixed(2)}${appState.system_info.latency_units}`;
}

function populateFilterOptions() {
  const prioritySelect = document.getElementById('filter-priority');
  const statusSelect = document.getElementById('filter-status');
  const typeSelect = document.getElementById('filter-type');
  const fillSelect = (select, options, selected) => {
    select.innerHTML = '';
    options.forEach((option) => {
      const opt = document.createElement('option');
      opt.value = option;
      opt.textContent = option.replace(/_/g, ' ');
      if (selected.includes(option)) opt.selected = true;
      select.appendChild(opt);
    });
  };
  fillSelect(prioritySelect, PRIORITY_OPTIONS, filters.priority);
  fillSelect(statusSelect, STATUS_OPTIONS, filters.status);
  const testTypes = Array.from(new Set(getAllTestsFromData(appState).map((t) => t.type)));
  fillSelect(typeSelect, testTypes, filters.type);
}

function applyFilters(tests) {
  return tests.filter((test) => {
    const matchesPriority = !filters.priority.length || filters.priority.includes(test.componentPriority);
    const matchesStatus = !filters.status.length || filters.status.includes(test.status);
    const matchesType = !filters.type.length || filters.type.includes(test.type);
    return matchesPriority && matchesStatus && matchesType;
  });
}

function sortTests(tests) {
  const sorted = [...tests];
  sorted.sort((a, b) => {
    const { column, direction } = sortState;
    const aValue = a[column] ?? '';
    const bValue = b[column] ?? '';
    if (typeof aValue === 'number' && typeof bValue === 'number') {
      return direction === 'asc' ? aValue - bValue : bValue - aValue;
    }
    const result = String(aValue).localeCompare(String(bValue), undefined, { sensitivity: 'base' });
    return direction === 'asc' ? result : -result;
  });
  return sorted;
}

function paginateTests(tests) {
  const totalPages = Math.max(1, Math.ceil(tests.length / pagination.perPage));
  if (pagination.page > totalPages) pagination.page = totalPages;
  const start = (pagination.page - 1) * pagination.perPage;
  const end = start + pagination.perPage;
  return { rows: tests.slice(start, end), totalPages };
}

function renderTable() {
  const tbody = document.querySelector('#tests-table tbody');
  tbody.innerHTML = '';
  const tests = sortTests(applyFilters(getAllTestsFromData(appState)));
  const { rows, totalPages } = paginateTests(tests);
  const template = document.getElementById('test-row-template');
  rows.forEach((test) => {
    const clone = template.content.firstElementChild.cloneNode(true);
    clone.dataset.testId = test.id;
    clone.querySelector('[data-field="id"]').textContent = test.id;
    clone.querySelector('[data-field="name"]').innerHTML = `<span class="cell-name">${sanitize(test.name)}</span>`;
    clone.querySelector('[data-field="type"]').textContent = test.type.replace(/_/g, ' ');
    const statusCell = clone.querySelector('[data-field="status"]');
    statusCell.textContent = test.status.replace(/_/g, ' ');
    statusCell.style.color = `var(--status-${test.status})`;
    clone.querySelector('[data-field="priority"]').textContent = test.componentPriority;
    clone.querySelector('[data-field="component"]').textContent = test.componentName;
    clone.querySelector('[data-field="latency"]').textContent = Number(test.actual_latency || 0).toFixed(2);
    clone.querySelector('[data-field="last_updated"]').textContent = formatDate(test.last_updated);
    clone.addEventListener('dragstart', (event) => {
      event.dataTransfer.setData('text/plain', test.id);
      event.dataTransfer.effectAllowed = 'move';
    });
    clone.addEventListener('click', (event) => {
      if (event.target.closest('input[type="checkbox"]')) return;
      clone.classList.toggle('selected');
    });
    clone.addEventListener('dblclick', () => openTestDialog(test));
    tbody.appendChild(clone);
  });
  document.getElementById('pagination-info').textContent = `Page ${pagination.page} of ${totalPages}`;
}

function renderSavedViews() {
  const container = document.getElementById('saved-views-list');
  container.innerHTML = '';
  savedViews.forEach((view, index) => {
    const div = document.createElement('div');
    div.className = 'saved-view';
    const button = document.createElement('button');
    button.textContent = view.name;
    button.addEventListener('click', () => {
      filters = deepClone(view.filters);
      pagination.page = 1;
      setTab(view.tab);
      populateFilterOptions();
      render();
    });
    const removeButton = document.createElement('button');
    removeButton.setAttribute('aria-label', `Remove ${view.name}`);
    removeButton.textContent = '×';
    removeButton.addEventListener('click', () => {
      savedViews.splice(index, 1);
      saveViews(savedViews);
      renderSavedViews();
    });
    div.append(button, removeButton);
    container.appendChild(div);
  });
}

function buildHierarchyData() {
  const root = { name: 'ColdVox', priority: 'critical', children: [] };
  Object.entries(appState.components).forEach(([, component]) => {
    const componentNode = {
      name: component.name,
      priority: component.priority,
      description: component.description,
      children: [],
    };
    Object.entries(component.subcomponents).forEach(([, sub]) => {
      componentNode.children.push({
        name: sub.name,
        priority: component.priority,
        files: sub.files,
        functions: sub.functions,
        children: sub.tests.map((test) => ({
          name: `${test.id}: ${test.name}`,
          priority: component.priority,
          status: test.status,
        })),
      });
    });
    root.children.push(componentNode);
  });
  return root;
}

function renderHierarchy() {
  const container = document.getElementById('hierarchy-chart');
  container.innerHTML = '';
  const rootData = d3.hierarchy(buildHierarchyData());
  const width = container.clientWidth || 640;
  const height = container.clientHeight || 360;
  const treeLayout = d3.tree().size([height, width - 160]);
  const root = treeLayout(rootData);
  const svg = d3
    .select(container)
    .append('svg')
    .attr('width', width)
    .attr('height', height)
    .attr('viewBox', `0 0 ${width} ${height}`)
    .attr('role', 'img')
    .attr('aria-label', 'Hierarchy of ColdVox components');

  svg
    .append('g')
    .attr('transform', 'translate(80,0)')
    .selectAll('path')
    .data(root.links())
    .enter()
    .append('path')
    .attr('fill', 'none')
    .attr('stroke', 'rgba(148, 163, 184, 0.4)')
    .attr('stroke-width', 1.5)
    .attr('d', d3.linkHorizontal().x((d) => d.y).y((d) => d.x));

  const node = svg
    .append('g')
    .attr('transform', 'translate(80,0)')
    .selectAll('g')
    .data(root.descendants())
    .enter()
    .append('g')
    .attr('transform', (d) => `translate(${d.y},${d.x})`);

  const priorityColor = (priority) => {
    switch (priority) {
      case 'critical':
        return 'var(--accent-red)';
      case 'high':
        return 'var(--accent-yellow)';
      case 'medium':
        return 'var(--accent-blue)';
      default:
        return 'var(--muted)';
    }
  };

  node
    .append('circle')
    .attr('r', 6)
    .attr('fill', (d) => priorityColor(d.data.priority))
    .attr('stroke', '#0f172a')
    .attr('stroke-width', 1.5);

  node
    .append('text')
    .attr('dy', 3)
    .attr('x', (d) => (d.children ? -12 : 12))
    .attr('text-anchor', (d) => (d.children ? 'end' : 'start'))
    .attr('fill', 'var(--text)')
    .attr('font-size', 12)
    .text((d) => d.data.name)
    .append('title')
    .text((d) => {
      const info = [];
      if (d.data.description) info.push(d.data.description);
      if (d.data.files) info.push(`Files: ${d.data.files.join(', ')}`);
      if (d.data.functions) info.push(`Functions: ${d.data.functions.join(', ')}`);
      if (d.data.status) info.push(`Status: ${d.data.status}`);
      return info.join('\n');
    });
}

async function renderMermaid() {
  const container = document.getElementById('mermaid-container');
  container.innerHTML = '';
  try {
    mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', theme: 'dark' });
    const { svg } = await mermaid.render('coldvox-flow', mermaidDefinition);
    container.innerHTML = svg;
  } catch (error) {
    console.error('Mermaid render failed', error);
    container.textContent = 'Unable to render pipeline diagram.';
  }
}

function renderDetails() {
  const container = document.getElementById('details-accordions');
  container.innerHTML = '';
  Object.entries(appState.components).forEach(([componentKey, component]) => {
    const details = document.createElement('details');
    details.className = 'accordion';
    details.setAttribute('data-component-key', componentKey);
    const summary = document.createElement('summary');
    summary.className = 'accordion__header';
    summary.innerHTML = `<span>${sanitize(component.name)}</span><span>${sanitize(component.priority.toUpperCase())}</span>`;
    details.appendChild(summary);
    const content = document.createElement('div');
    content.className = 'accordion__content';

    const meta = document.createElement('div');
    meta.className = 'component-meta';
    const description = document.createElement('p');
    description.textContent = component.description;
    const progressWrap = document.createElement('div');
    progressWrap.className = 'progress-radial';
    const canvas = document.createElement('canvas');
    canvas.width = 140;
    canvas.height = 140;
    canvas.dataset.componentKey = componentKey;
    progressWrap.appendChild(canvas);
    meta.append(description, progressWrap);
    content.appendChild(meta);

    Object.entries(component.subcomponents).forEach(([subKey, sub]) => {
      const subDiv = document.createElement('div');
      subDiv.className = 'component-tests';
      const header = document.createElement('h4');
      header.textContent = sub.name;
      subDiv.appendChild(header);
      const table = document.createElement('table');
      table.innerHTML = `
        <thead><tr><th>ID</th><th>Name</th><th>Status</th><th>Coverage</th><th>Link</th><th>Notes</th></tr></thead>
        <tbody></tbody>
      `;
      const tbody = table.querySelector('tbody');
      sub.tests.forEach((test) => {
        const tr = document.createElement('tr');
        tr.dataset.testId = test.id;
        tr.innerHTML = `
          <td>${sanitize(test.id)}</td>
          <td>${sanitize(test.name)}</td>
          <td>
            <select class="inline-status" aria-label="Status for ${sanitize(test.id)}">
              ${STATUS_OPTIONS.map((status) => `<option value="${status}" ${status === test.status ? 'selected' : ''}>${status.replace(/_/g, ' ')}</option>`).join('')}
            </select>
          </td>
          <td>
            <input type="number" class="inline-coverage" min="0" max="100" value="${test.coverage_actual ?? test.coverage_target}" aria-label="Coverage actual for ${sanitize(test.id)}" />
            / ${test.coverage_target}
          </td>
          <td>
            <input type="url" class="inline-link" value="${sanitize(test.link)}" placeholder="https://" aria-label="Link for ${sanitize(test.id)}" />
          </td>
          <td>
            <textarea class="inline-notes" rows="2" aria-label="Notes for ${sanitize(test.id)}">${sanitize(test.notes || '')}</textarea>
          </td>
        `;
        tbody.appendChild(tr);
      });
      subDiv.appendChild(table);
      content.appendChild(subDiv);
    });

    details.appendChild(content);
    container.appendChild(details);
  });
  renderComponentProgress();
}

function renderComponentProgress() {
  Object.entries(appState.components).forEach(([componentKey, component]) => {
    const canvas = document.querySelector(`canvas[data-component-key="${componentKey}"]`);
    if (!canvas) return;
    const tests = [];
    Object.values(component.subcomponents).forEach((sub) => {
      sub.tests.forEach((test) => tests.push(test));
    });
    const passed = tests.filter((test) => test.status === 'passed').length;
    const data = [passed, tests.length - passed];
    const ctx = canvas.getContext('2d');
    if (charts[`component-${componentKey}`]) {
      charts[`component-${componentKey}`].destroy();
    }
    charts[`component-${componentKey}`] = new Chart(ctx, {
      type: 'doughnut',
      data: {
        labels: ['Passed', 'Remaining'],
        datasets: [
          {
            data,
            backgroundColor: [getComputedStyle(document.documentElement).getPropertyValue('--accent-green'), 'rgba(148,163,184,0.2)'],
            borderWidth: 0,
            circumference: 360,
            cutout: '70%',
          },
        ],
      },
      options: {
        plugins: {
          legend: { display: false },
          tooltip: { callbacks: { label: (context) => `${context.label}: ${context.formattedValue}` } },
        },
      },
    });
  });
}

function renderAnalytics() {
  const stats = computeStats(appState);
  const labels = STATUS_OPTIONS.map((status) => status.replace(/_/g, ' '));
  const data = STATUS_OPTIONS.map((status) => stats.statusCounts[status] || 0);
  const pieCtx = document.getElementById('status-pie');
  if (charts.statusPie) charts.statusPie.destroy();
  charts.statusPie = new Chart(pieCtx, {
    type: 'pie',
    data: {
      labels,
      datasets: [
        {
          data,
          backgroundColor: STATUS_OPTIONS.map((status) => getComputedStyle(document.documentElement).getPropertyValue(`--status-${status}`) || 'rgba(148,163,184,0.3)'),
        },
      ],
    },
    options: {
      plugins: { legend: { position: 'bottom' } },
    },
  });

  const componentLabels = Object.keys(stats.byComponent);
  const componentData = componentLabels.map((label) => {
    const entry = stats.byComponent[label];
    return entry.total ? Math.round((entry.passed / entry.total) * 100) : 0;
  });
  const barCtx = document.getElementById('component-bar');
  if (charts.componentBar) charts.componentBar.destroy();
  charts.componentBar = new Chart(barCtx, {
    type: 'bar',
    data: {
      labels: componentLabels,
      datasets: [
        {
          label: 'Pass %',
          data: componentData,
          backgroundColor: componentLabels.map((label) => {
            const priority = stats.byComponent[label].priority;
            switch (priority) {
              case 'critical':
                return 'rgba(239, 68, 68, 0.65)';
              case 'high':
                return 'rgba(245, 158, 11, 0.65)';
              case 'medium':
                return 'rgba(59, 130, 246, 0.65)';
              default:
                return 'rgba(148, 163, 184, 0.65)';
            }
          }),
        },
      ],
    },
    options: {
      scales: {
        y: { beginAtZero: true, max: 100, ticks: { color: 'var(--muted)' } },
        x: { ticks: { color: 'var(--muted)' } },
      },
      plugins: { legend: { display: false } },
    },
  });

  const trendCtx = document.getElementById('status-line');
  const historyPoints = [];
  getAllTestsFromData(appState).forEach((test) => {
    (test.history || []).forEach((entry) => {
      historyPoints.push({ date: entry.date, status: entry.status });
    });
  });
  historyPoints.sort((a, b) => new Date(a.date) - new Date(b.date));
  const cumulative = [];
  const counts = Object.fromEntries(STATUS_OPTIONS.map((status) => [status, 0]));
  historyPoints.forEach((entry) => {
    counts[entry.status] += 1;
    cumulative.push({ date: entry.date, ...counts });
  });
  if (charts.statusLine) charts.statusLine.destroy();
  charts.statusLine = new Chart(trendCtx, {
    type: 'line',
    data: {
      datasets: STATUS_OPTIONS.map((status) => ({
        label: status.replace(/_/g, ' '),
        data: cumulative.map((point) => ({ x: point.date, y: point[status] })),
        borderColor: getComputedStyle(document.documentElement).getPropertyValue(`--status-${status}`) || '#fff',
        tension: 0.4,
      })),
    },
    options: {
      parsing: false,
      scales: {
        x: { type: 'time', ticks: { color: 'var(--muted)' } },
        y: { beginAtZero: true, ticks: { color: 'var(--muted)' } },
      },
      plugins: { legend: { position: 'bottom' } },
    },
  });

  const gaugeCtx = document.getElementById('latency-gauge');
  if (charts.latencyGauge) charts.latencyGauge.destroy();
  charts.latencyGauge = new Chart(gaugeCtx, {
    type: 'doughnut',
    data: {
      labels: ['Actual', 'Remaining'],
      datasets: [
        {
          data: [stats.avgLatency, Math.max(0, appState.system_info.target_latency - stats.avgLatency)],
          backgroundColor: ['rgba(34,197,94,0.8)', 'rgba(148,163,184,0.2)'],
          borderWidth: 0,
          circumference: 180,
          rotation: 270,
          cutout: '70%',
        },
      ],
    },
    options: {
      plugins: {
        legend: { display: false },
        tooltip: { callbacks: { label: (context) => `${context.label}: ${context.formattedValue}s` } },
      },
    },
  });

  renderHeatmap();
}

function renderHeatmap() {
  const container = document.getElementById('heatmap');
  container.innerHTML = '';
  const tests = getAllTestsFromData(appState);
  const components = Array.from(new Set(tests.map((t) => t.componentName)));
  const types = Array.from(new Set(tests.map((t) => t.type)));
  const width = container.clientWidth || 640;
  const height = 60 + components.length * 40;
  const svg = d3
    .select(container)
    .append('svg')
    .attr('width', width)
    .attr('height', height)
    .attr('viewBox', `0 0 ${width} ${height}`);

  const xScale = d3.scaleBand().domain(types).range([160, width - 20]).padding(0.1);
  const yScale = d3.scaleBand().domain(components).range([30, height - 20]).padding(0.1);

  const colorScale = d3.scaleLinear().domain([0, 0.5, 1]).range(['#1f2937', '#3b82f6', '#22c55e']);

  const patternDefs = svg.append('defs');
  types.forEach((type, idx) => {
    const pattern = patternDefs
      .append('pattern')
      .attr('id', `pattern-${type}`)
      .attr('patternUnits', 'userSpaceOnUse')
      .attr('width', 6)
      .attr('height', 6)
      .attr('patternTransform', `rotate(${idx % 2 === 0 ? 45 : -45})`);
    pattern
      .append('rect')
      .attr('width', 6)
      .attr('height', 6)
      .attr('fill', TYPE_COLORS[type] || '#64748b');
    pattern
      .append('line')
      .attr('x1', 0)
      .attr('y1', 0)
      .attr('x2', 0)
      .attr('y2', 6)
      .attr('stroke', 'rgba(255,255,255,0.4)')
      .attr('stroke-width', 1);
  });

  components.forEach((component) => {
    svg
      .append('text')
      .attr('x', 10)
      .attr('y', yScale(component) + yScale.bandwidth() / 2 + 4)
      .attr('fill', 'var(--text)')
      .attr('font-size', 12)
      .text(component);
  });

  types.forEach((type) => {
    svg
      .append('text')
      .attr('x', xScale(type) + xScale.bandwidth() / 2)
      .attr('y', 16)
      .attr('fill', 'var(--text)')
      .attr('text-anchor', 'middle')
      .attr('font-size', 12)
      .text(type.replace(/_/g, ' '));
  });

  components.forEach((component) => {
    types.forEach((type) => {
      const relevant = tests.filter((test) => test.componentName === component && test.type === type);
      const ratio = relevant.length ? relevant.filter((test) => test.status === 'passed').length / relevant.length : 0;
      svg
        .append('rect')
        .attr('x', xScale(type))
        .attr('y', yScale(component))
        .attr('width', xScale.bandwidth())
        .attr('height', yScale.bandwidth())
        .attr('fill', ratio ? colorScale(ratio) : `url(#pattern-${type})`)
        .attr('opacity', ratio ? 1 : 0.7)
        .append('title')
        .text(`${component} × ${type}: ${Math.round(ratio * 100)}% passed`);
    });
  });
}

function findTestById(id) {
  for (const component of Object.values(appState.components)) {
    for (const sub of Object.values(component.subcomponents)) {
      const test = sub.tests.find((t) => t.id === id);
      if (test) return { test, collection: sub.tests };
    }
  }
  const remaining = appState.tests_remaining.find((t) => t.id === id);
  if (remaining) return { test: remaining, collection: appState.tests_remaining };
  return null;
}

function updateTestStatus(id, newStatus, options = {}) {
  const match = findTestById(id);
  if (!match) return;
  const { test } = match;
  if (newStatus === 'passed' && !meetsCoverage(test)) {
    alert(`Cannot set ${id} to passed. Coverage target ${test.coverage_target}% not met.`);
    return;
  }
  if ((newStatus === 'failed' || newStatus === 'blocked') && !options.reason) {
    const reason = prompt('Provide a brief reason for the status change:');
    if (!reason) return;
    test.notes = `${reason}\n${test.notes || ''}`.trim();
  }
  test.status = newStatus;
  const now = new Date().toISOString();
  test.last_updated = now;
  test.history = test.history || [];
  test.history.push({ date: now, status: newStatus });
  updateFlakiness(test);
  saveState(appState);
  render();
}

function handleDragTargets() {
  document.querySelectorAll('.drag-target').forEach((target) => {
    target.addEventListener('dragover', (event) => {
      event.preventDefault();
      target.classList.add('drag-over');
    });
    target.addEventListener('dragleave', () => {
      target.classList.remove('drag-over');
    });
    target.addEventListener('drop', (event) => {
      event.preventDefault();
      target.classList.remove('drag-over');
      const id = event.dataTransfer.getData('text/plain');
      if (id) {
        updateTestStatus(id, target.dataset.status);
      }
    });
  });
}

function openTestDialog(existingTest) {
  const dialog = document.getElementById('test-dialog');
  const form = document.getElementById('test-form');
  document.getElementById('dialog-title').textContent = existingTest ? `Edit ${existingTest.id}` : 'New Test';
  const componentSelect = document.getElementById('test-component');
  componentSelect.innerHTML = '';
  Object.entries(appState.components).forEach(([key, component]) => {
    const opt = document.createElement('option');
    opt.value = key;
    opt.textContent = component.name;
    componentSelect.appendChild(opt);
  });
  const statuses = document.getElementById('test-status');
  statuses.innerHTML = STATUS_OPTIONS.map((status) => `<option value="${status}">${status.replace(/_/g, ' ')}</option>`).join('');
  const prioritySelect = document.getElementById('test-priority');
  prioritySelect.innerHTML = PRIORITY_OPTIONS.map((priority) => `<option value="${priority}">${priority}</option>`).join('');
  if (existingTest) {
    document.getElementById('test-id').value = existingTest.id;
    document.getElementById('test-id').disabled = true;
    document.getElementById('test-name').value = existingTest.name;
    document.getElementById('test-description').value = existingTest.description;
    document.getElementById('test-type').value = existingTest.type;
    document.getElementById('test-status').value = existingTest.status;
    document.getElementById('test-priority').value = existingTest.componentPriority;
    document.getElementById('test-latency').value = existingTest.actual_latency;
    document.getElementById('test-component').value = existingTest.componentKey;
    document.getElementById('test-coverage').value = existingTest.coverage_actual ?? existingTest.coverage_target ?? 0;
    document.getElementById('test-link').value = existingTest.link || '';
    document.getElementById('test-notes').value = existingTest.notes || '';
  } else {
    form.reset();
    document.getElementById('test-id').disabled = false;
    document.getElementById('test-coverage').value = 0;
  }
  dialog.returnValue = '';
  dialog.showModal();

  const confirmHandler = () => {
    const formData = new FormData(form);
    const id = formData.get('testId');
    if (!id) return;
    const match = findTestById(id);
    if (match) {
      match.test.name = formData.get('name');
      match.test.description = formData.get('description');
      match.test.type = formData.get('type');
      match.test.coverage_actual = Number(formData.get('coverage')) || match.test.coverage_actual;
      const newStatus = formData.get('status');
      if (newStatus === 'passed' && !meetsCoverage(match.test)) {
        alert('Coverage target not met. Update coverage before marking as passed.');
        return;
      }
      match.test.status = newStatus;
      match.test.actual_latency = Number(formData.get('latency'));
      match.test.notes = formData.get('notes');
      match.test.link = formData.get('link') || match.test.link;
      match.test.last_updated = new Date().toISOString();
      match.test.history.push({ date: match.test.last_updated, status: match.test.status });
      updateFlakiness(match.test);
    } else {
      const componentKey = formData.get('component');
      const component = appState.components[componentKey];
      const firstSubKey = Object.keys(component.subcomponents)[0];
      const subTests = component.subcomponents[firstSubKey].tests;
      const newTest = {
        id,
        name: formData.get('name'),
        description: formData.get('description'),
        type: formData.get('type'),
        status: formData.get('status'),
        coverage_target: 80,
       coverage_actual: Number(formData.get('coverage')) || 0,
        link: formData.get('link') || '',
        actual_latency: Number(formData.get('latency')),
        last_updated: new Date().toISOString(),
        repo: '',
        issue: '',
        pr: '',
        workflow_url: '',
        flaky: false,
        history: [{ date: new Date().toISOString(), status: formData.get('status') }],
        notes: formData.get('notes') || '',
      };
      subTests.push(newTest);
    }
    saveState(appState);
    render();
    dialog.close();
    dialog.querySelector('button[value="confirm"]').removeEventListener('click', confirmHandler);
  };

  dialog.querySelector('button[value="confirm"]').addEventListener('click', confirmHandler);
  dialog.addEventListener('close', () => {
    form.reset();
    document.getElementById('test-id').disabled = false;
    dialog.querySelector('button[value="confirm"]').removeEventListener('click', confirmHandler);
  }, { once: true });
}

function attachTableHandlers() {
  document.querySelectorAll('#tests-table thead th').forEach((th) => {
    th.addEventListener('click', () => {
      const column = th.dataset.sort;
      if (sortState.column === column) {
        sortState.direction = sortState.direction === 'asc' ? 'desc' : 'asc';
      } else {
        sortState.column = column;
        sortState.direction = 'asc';
      }
      document.querySelectorAll('#tests-table thead th').forEach((header) => header.classList.remove('sorted-desc'));
      if (sortState.direction === 'desc') th.classList.add('sorted-desc');
      renderTable();
    });
  });

  document.getElementById('prev-page').addEventListener('click', () => {
    pagination.page = Math.max(1, pagination.page - 1);
    renderTable();
  });
  document.getElementById('next-page').addEventListener('click', () => {
    pagination.page += 1;
    renderTable();
  });

  document.getElementById('select-all').addEventListener('change', (event) => {
    const checked = event.target.checked;
    document.querySelectorAll('#tests-table tbody tr').forEach((row) => {
      row.classList.toggle('selected', checked);
    });
  });

  document.querySelectorAll('.status-pill').forEach((button) => {
    button.addEventListener('click', () => {
      const selectedRows = Array.from(document.querySelectorAll('#tests-table tbody tr.selected'));
      selectedRows.forEach((row) => updateTestStatus(row.dataset.testId, button.dataset.status));
    });
  });
}

function attachSidebarHandlers() {
  document.getElementById('filter-priority').addEventListener('change', (event) => {
    filters.priority = Array.from(event.target.selectedOptions).map((opt) => opt.value);
    pagination.page = 1;
    render();
  });
  document.getElementById('filter-status').addEventListener('change', (event) => {
    filters.status = Array.from(event.target.selectedOptions).map((opt) => opt.value);
    pagination.page = 1;
    render();
  });
  document.getElementById('filter-type').addEventListener('change', (event) => {
    filters.type = Array.from(event.target.selectedOptions).map((opt) => opt.value);
    pagination.page = 1;
    render();
  });
  document.getElementById('clear-filters').addEventListener('click', () => {
    filters = { priority: [], status: [], type: [] };
    pagination.page = 1;
    populateFilterOptions();
    render();
  });
  document.getElementById('save-current-view').addEventListener('click', () => {
    const name = prompt('Name for this view:');
    if (!name) return;
    const existing = savedViews.find((view) => view.name === name);
    const payload = { name, filters: deepClone(filters), tab: currentTab };
    if (existing) {
      Object.assign(existing, payload);
    } else {
      savedViews.push(payload);
    }
    saveViews(savedViews);
    renderSavedViews();
  });
}

function attachHeaderHandlers() {
  document.querySelectorAll('.tab-button').forEach((button) => {
    button.addEventListener('click', () => setTab(button.dataset.tab));
  });
  const search = document.getElementById('global-search');
  search.addEventListener('input', debounce((event) => {
    const value = event.target.value.trim().toLowerCase();
    const tests = getAllTestsFromData(appState).filter((test) => test.id.toLowerCase().includes(value) || test.name.toLowerCase().includes(value));
    if (value) {
      filters = { priority: [], status: [], type: [] };
      pagination.page = 1;
      populateFilterOptions();
      renderTableWithData(tests);
    } else {
      render();
    }
  }, 200));
  document.addEventListener('keydown', (event) => {
    if (event.key === '/') {
      event.preventDefault();
      search.focus();
    }
    if (event.key === 'g' && event.target === document.body) {
      const handler = (evt) => {
        if (evt.key === 'a') setTab('architecture');
        if (evt.key === 'd') setTab('dashboard');
        if (evt.key === 't') setTab('details');
      };
      document.addEventListener('keydown', handler, { once: true });
    }
    if (/^[1-6]$/.test(event.key)) {
      const status = STATUS_OPTIONS[Number(event.key) - 1];
      const selectedRows = Array.from(document.querySelectorAll('#tests-table tbody tr.selected'));
      selectedRows.forEach((row) => updateTestStatus(row.dataset.testId, status));
    }
    if (event.key.toLowerCase() === 'e') {
      const firstSelected = document.querySelector('#tests-table tbody tr.selected');
      if (firstSelected) {
        const match = findTestById(firstSelected.dataset.testId);
        if (match) openTestDialog(match.test);
      }
    }
  });

  document.getElementById('demo-data-btn').addEventListener('click', () => {
    applyDemoData();
    render();
  });
}

function renderTableWithData(customRows) {
  const tbody = document.querySelector('#tests-table tbody');
  tbody.innerHTML = '';
  const template = document.getElementById('test-row-template');
  customRows.forEach((test) => {
    const clone = template.content.firstElementChild.cloneNode(true);
    clone.dataset.testId = test.id;
    clone.querySelector('[data-field="id"]').textContent = test.id;
    clone.querySelector('[data-field="name"]').textContent = test.name;
    clone.querySelector('[data-field="type"]').textContent = test.type;
    clone.querySelector('[data-field="status"]').textContent = test.status;
    clone.querySelector('[data-field="priority"]').textContent = test.componentPriority;
    clone.querySelector('[data-field="component"]').textContent = test.componentName;
    clone.querySelector('[data-field="latency"]').textContent = Number(test.actual_latency || 0).toFixed(2);
    clone.querySelector('[data-field="last_updated"]').textContent = formatDate(test.last_updated);
    tbody.appendChild(clone);
  });
}

function debounce(fn, delay) {
  let timeout;
  return (...args) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => fn(...args), delay);
  };
}

function attachDetailsHandlers() {
  document.getElementById('details-accordions').addEventListener('change', (event) => {
    const row = event.target.closest('tr');
    if (!row) return;
    const match = findTestById(row.dataset.testId);
    if (!match) return;
    if (event.target.classList.contains('inline-status')) {
      const newStatus = event.target.value;
      if (newStatus === 'passed' && !meetsCoverage(match.test)) {
        alert('Coverage target not met. Increase coverage before marking as passed.');
        event.target.value = match.test.status;
        return;
      }
      if ((newStatus === 'failed' || newStatus === 'blocked') && !match.test.notes) {
        const reason = prompt('Provide a brief reason for this status change:');
        if (!reason) {
          event.target.value = match.test.status;
          return;
        }
        match.test.notes = `${reason}\n${match.test.notes || ''}`.trim();
        row.querySelector('.inline-notes').value = match.test.notes;
      }
      updateTestStatus(match.test.id, newStatus, { reason: true });
    }
    if (event.target.classList.contains('inline-coverage')) {
      match.test.coverage_actual = Number(event.target.value);
      saveState(appState);
      render();
    }
    if (event.target.classList.contains('inline-link')) {
      match.test.link = event.target.value.trim();
      saveState(appState);
    }
  });
  document.getElementById('details-accordions').addEventListener('input', (event) => {
    if (event.target.classList.contains('inline-notes')) {
      const row = event.target.closest('tr');
      if (!row) return;
      const match = findTestById(row.dataset.testId);
      if (!match) return;
      match.test.notes = event.target.value;
      saveState(appState);
    }
  });
}

function attachFooterHandlers() {
  document.getElementById('export-json').addEventListener('click', () => {
    const blob = new Blob([JSON.stringify(appState, null, 2)], { type: 'application/json' });
    downloadBlob(blob, 'coldvox-dashboard.json');
  });
  document.getElementById('export-csv').addEventListener('click', () => {
    const data = getAllTestsFromData(appState).map((test) => ({
      id: test.id,
      name: test.name,
      type: test.type,
      status: test.status,
      priority: test.componentPriority,
      component: test.componentName,
      latency: test.actual_latency,
      last_updated: test.last_updated,
    }));
    const csv = Papa.unparse(data);
    downloadBlob(new Blob([csv], { type: 'text/csv' }), 'coldvox-dashboard.csv');
  });
  document.getElementById('export-pdf').addEventListener('click', async () => {
    const { jsPDF } = window.jspdf;
    const doc = new jsPDF('landscape');
    doc.setFontSize(16);
    doc.text('ColdVox Coverage Dashboard', 14, 20);
    doc.setFontSize(11);
    const stats = computeStats(appState);
    doc.text(`Overall Coverage: ${stats.completed}/${stats.total}`, 14, 30);
    doc.text(`Average Latency: ${stats.avgLatency.toFixed(2)}${appState.system_info.latency_units}`, 14, 38);
    const svg = document.querySelector('#mermaid-container svg');
    if (svg) {
      const serializer = new XMLSerializer();
      const svgBlob = new Blob([serializer.serializeToString(svg)], { type: 'image/svg+xml' });
      const url = URL.createObjectURL(svgBlob);
      const img = new Image();
      img.onload = () => {
        doc.addImage(img, 'PNG', 14, 50, 120, 60);
        doc.save('coldvox-dashboard.pdf');
        URL.revokeObjectURL(url);
      };
      img.src = url;
    } else {
      doc.save('coldvox-dashboard.pdf');
    }
  });
  document.getElementById('import-json-btn').addEventListener('click', () => {
    document.getElementById('import-json').click();
  });
  document.getElementById('import-json').addEventListener('change', (event) => {
    const file = event.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const imported = migrateState(JSON.parse(reader.result));
        appState = imported;
        saveState(appState);
        render();
      } catch (error) {
        alert('Import failed: invalid JSON');
      }
    };
    reader.readAsText(file);
  });
}

function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function applyDemoData() {
  const tests = getAllTestsFromData(appState);
  tests.forEach((test) => {
    const statuses = ['pending', 'in_progress', 'passed', 'failed', 'blocked'];
    test.status = statuses[Math.floor(Math.random() * statuses.length)];
    test.actual_latency = Number((Math.random() * 1.5).toFixed(2));
    test.coverage_actual = Math.min(100, Math.floor(Math.random() * 20) + test.coverage_target - 10);
    const now = new Date().toISOString();
    test.last_updated = now;
    test.history.push({ date: now, status: test.status });
    updateFlakiness(test);
  });
  saveState(appState);
}

function maybeVirtualize() {
  const tbody = document.querySelector('#tests-table tbody');
  const total = getAllTestsFromData(appState).length;
  if (total <= 200) return;
  const rowHeight = 48;
  const viewportHeight = tbody.parentElement.clientHeight;
  const visibleCount = Math.ceil(viewportHeight / rowHeight) + 5;
  let start = 0;
  const onScroll = () => {
    const scrollTop = tbody.parentElement.scrollTop;
    start = Math.floor(scrollTop / rowHeight);
    const tests = sortTests(applyFilters(getAllTestsFromData(appState)));
    const slice = tests.slice(start, start + visibleCount);
    renderTableWithData(slice);
    tbody.style.paddingTop = `${start * rowHeight}px`;
    tbody.style.paddingBottom = `${Math.max(0, tests.length - start - slice.length) * rowHeight}px`;
  };
  tbody.parentElement.addEventListener('scroll', onScroll);
}

function render() {
  updateSummary();
  if (currentTab === 'dashboard') {
    renderTable();
  }
  if (currentTab === 'details') {
    renderDetails();
  }
  if (currentTab === 'architecture') {
    renderHierarchy();
    renderMermaid();
  }
  if (currentTab === 'analytics') {
    renderAnalytics();
  }
}

function init() {
  applyHash();
  populateFilterOptions();
  renderSavedViews();
  updateSummary();
  render();
  attachTableHandlers();
  attachSidebarHandlers();
  attachHeaderHandlers();
  attachDetailsHandlers();
  attachFooterHandlers();
  handleDragTargets();
  maybeVirtualize();
}

window.addEventListener('hashchange', () => {
  applyHash();
  render();
});

document.addEventListener('DOMContentLoaded', init);

