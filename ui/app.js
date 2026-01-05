// Safe string truncation helper to respect UTF-8 boundaries
function safeTruncate(str, maxLength, suffix = '…') {
  if (str.length <= maxLength) {
    return str;
  }
  // Use Array.from to properly count UTF-8 characters
  const chars = Array.from(str);
  if (chars.length <= maxLength) {
    return str;
  }
  const suffixChars = Array.from(suffix);
  const maxContent = maxLength - suffixChars.length;
  return chars.slice(0, Math.max(0, maxContent)).join('') + suffix;
}

// Escape HTML special characters
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Highlight search matches in text (returns HTML string)
function highlightMatches(text, query) {
  if (!query || query.length < 2) {
    return escapeHtml(text);
  }

  const escaped = escapeHtml(text);
  const escapedQuery = escapeHtml(query);

  // Case-insensitive search and highlight
  const regex = new RegExp(`(${escapedQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
  return escaped.replace(regex, '<mark class="search-highlight">$1</mark>');
}

const navItems = document.querySelectorAll('.nav-item');
const views = document.querySelectorAll('.view');
const viewTitle = document.getElementById('view-title');
const projectName = document.getElementById('project-name');
const projectPathInput = document.getElementById('project-path');
const homeDirectoryInput = document.getElementById('home-directory');
const sidebarStatus = document.getElementById('sidebar-status');
const statusText = document.getElementById('status-text');
const blameFileList = document.getElementById('blame-file-list');
const blameCodeBlock = document.getElementById('blame-code-block');
const blameDetailTitle = document.getElementById('blame-detail-title');
const blameDetailModel = document.getElementById('blame-detail-model');
const blameDetailTimestamp = document.getElementById('blame-detail-timestamp');
const blameDetailTrace = document.getElementById('blame-detail-trace');
const picker = document.getElementById('project-picker');
const pickerList = document.getElementById('picker-list');
const pickerSearch = document.getElementById('picker-search');
const pickerManual = document.getElementById('picker-manual');
const pickerClose = document.getElementById('picker-close');
const blameDetailsPanel = document.getElementById('blame-details-panel');
const closeDetailsBtn = document.getElementById('close-details');
const agentTouchedOnlyCheckbox = document.getElementById('agent-touched-only');
const fileSearchInput = document.getElementById('file-search');

// Track state for filtering
let allFiles = [];
let agentTouchedFiles = new Set();

const titleMap = {
  blame: 'Blame Viewer',
  timeline: 'Timeline',
  transcripts: 'Transcript Viewer',
  settings: 'Settings',
};

// Transcript viewer state
let transcriptsList = [];
let currentTranscript = null;
let transcriptSearchQuery = '';  // For in-transcript filtering
let crossSessionSearchQuery = '';  // For cross-session search
let searchResults = null;  // Store search results with snippets
const transcriptList = document.getElementById('transcript-list');
const transcriptMessages = document.getElementById('transcript-messages');
const transcriptMetaSession = document.getElementById('transcript-meta-session');
const transcriptMetaAgent = document.getElementById('transcript-meta-agent');
const transcriptMetaTime = document.getElementById('transcript-meta-time');
const transcriptSearch = document.getElementById('transcript-search');
const transcriptListSearch = document.getElementById('transcript-list-search');
const transcriptSearchInfo = document.getElementById('transcript-search-info');

const RECENT_PROJECTS_KEY = 'recentProjects';
const MAX_RECENT_PROJECTS = 6;
const DEFAULT_PROJECTS = [];
let pickerProjects = DEFAULT_PROJECTS;

const HOME_DIRECTORY_KEY = 'homeDirectory';

function setStatus(message) {
  if (statusText) {
    statusText.textContent = message;
  }
  if (sidebarStatus) {
    sidebarStatus.textContent = message;
  }
}

function setProject(pathOrName) {
  if (projectName) {
    projectName.textContent = pathOrName || '—';
  }
  if (projectPathInput && pathOrName) {
    projectPathInput.value = pathOrName;
  }
}

function navigate(view) {
  const button = Array.from(navItems).find((b) => b.dataset.view === view);
  if (button) {
    button.click();
  }
}

// Navigate to blame viewer and load a specific file
async function navigateToBlamFile(filePath) {
  navigate('blame');

  // Wait for view to switch, then load files and select the target file
  await new Promise(resolve => setTimeout(resolve, 100));

  await loadProjectFilesForBlame();

  // Find and click the file in the list
  const fileItems = document.querySelectorAll('#blame-file-list li[data-file]');
  const targetItem = Array.from(fileItems).find(item =>
    item.getAttribute('data-file') === filePath
  );

  if (targetItem) {
    targetItem.click();
    setStatus(`Viewing: ${filePath}`);
  }
}

// Navigate to transcript viewer and load a specific session
async function navigateToTranscript(sessionId) {
  navigate('transcripts');

  // Wait for view to switch, then load transcripts and select the target
  await new Promise(resolve => setTimeout(resolve, 100));

  await loadTranscriptsList();

  // Find and click the transcript in the list
  const transcriptItems = document.querySelectorAll('#transcript-list li[data-session]');
  const targetItem = Array.from(transcriptItems).find(item =>
    item.getAttribute('data-session') === sessionId
  );

  if (targetItem) {
    targetItem.click();
    setStatus(`Viewing transcript: ${sessionId.slice(0, 8)}...`);
  } else {
    // Fallback: load transcript directly by session ID
    await loadTranscript(sessionId);
    setStatus(`Viewing transcript: ${sessionId.slice(0, 8)}...`);
  }
}

function getProjectDir() {
  return projectPathInput?.value?.trim() || null;
}

function clearBlameDetails() {
  if (blameDetailTitle) blameDetailTitle.textContent = 'Select a line';
  if (blameDetailModel) blameDetailModel.textContent = 'Model: —';
  if (blameDetailTimestamp) blameDetailTimestamp.textContent = 'Timestamp: —';
  if (blameDetailTrace) blameDetailTrace.textContent = 'Session: —';
}

function getVisibleFiles() {
  const showOnlyAgentTouched = agentTouchedOnlyCheckbox?.checked ?? true;
  const searchQuery = (fileSearchInput?.value ?? '').toLowerCase().trim();

  let filtered = allFiles;

  // Apply agent-touched filter
  if (showOnlyAgentTouched) {
    filtered = filtered.filter((f) => agentTouchedFiles.has(f));
  }

  // Apply search filter
  if (searchQuery) {
    filtered = filtered.filter((f) => f.toLowerCase().includes(searchQuery));
  }

  return filtered;
}

function renderFileList() {
  if (!blameFileList) return;
  blameFileList.textContent = '';

  const visibleFiles = getVisibleFiles();

  if (visibleFiles.length === 0) {
    const li = document.createElement('li');
    li.className = 'muted';
    const searchQuery = (fileSearchInput?.value ?? '').trim();
    if (searchQuery) {
      li.textContent = `No files matching "${searchQuery}".`;
    } else if (agentTouchedOnlyCheckbox?.checked) {
      li.textContent = 'No agent-touched files found.';
    } else {
      li.textContent = 'No files found (or folder is empty).';
    }
    blameFileList.appendChild(li);
    return;
  }

  visibleFiles.forEach((file) => {
    const li = document.createElement('li');
    li.textContent = file;
    li.setAttribute('data-file', file);  // Add data attribute for programmatic selection
    li.addEventListener('click', async () => {
      blameFileList
        .querySelectorAll('li.active')
        .forEach((el) => el.classList.remove('active'));
      li.classList.add('active');

      const projectDir = getProjectDir();
      setStatus(`Computing blame: ${file}…`);
      clearBlameDetails();
      if (blameCodeBlock) blameCodeBlock.textContent = '';

      try {
        const invoke = window.__TAURI__?.invoke;
        if (!invoke) {
          setStatus('Blame requires the desktop app (Tauri).');
          return;
        }
        const blamed = await invoke('blame_file', { projectDir, filePath: file });
        renderBlameLines(blamed?.lines ?? []);
        setStatus(`Blame ready: ${file} (${blamed?.line_count ?? 0} lines)`);
      } catch (err) {
        console.error(err);
        setStatus('Blame failed. See console for details.');
      }
    });
    blameFileList.appendChild(li);
  });
}

function renderBlameLines(lines) {
  if (!blameCodeBlock) return;
  blameCodeBlock.textContent = '';

  if (!lines || lines.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'muted';
    empty.textContent = 'No blame data yet (no matching edits found in traces).';
    blameCodeBlock.appendChild(empty);
    return;
  }

  lines.forEach((line) => {
    const row = document.createElement('div');
    row.className = 'code-line';
    row.dataset.lineNo = String(line.line_no);

    const no = document.createElement('span');
    no.textContent = String(line.line_no);

    const badge = document.createElement('span');
    badge.className = 'badge';
    const model = line.meta?.model || 'Unknown';
    if (!line.meta) badge.classList.add('neutral');
    badge.textContent = line.meta ? model.split(/[\\s-]/)[0] : 'Unknown';

    const code = document.createElement('code');
    code.textContent = line.text;

    row.appendChild(no);
    row.appendChild(badge);
    row.appendChild(code);

    row.addEventListener('click', () => {
      document
        .querySelectorAll('.code-line.is-selected')
        .forEach((el) => el.classList.remove('is-selected'));
      row.classList.add('is-selected');

      if (blameDetailTitle) blameDetailTitle.textContent = `Line ${line.line_no}`;
      if (blameDetailModel) blameDetailModel.textContent = `Model: ${model}`;
      if (blameDetailTimestamp)
        blameDetailTimestamp.textContent = `Timestamp: ${line.meta?.timestamp ?? '—'}`;

      // Make session ID clickable to view transcript
      if (blameDetailTrace) {
        const sessionId = line.meta?.session_id;
        if (sessionId && sessionId !== '—') {
          blameDetailTrace.innerHTML = `Session: <a href="#" class="blame-session-link" data-session="${sessionId.replace(/"/g, '&quot;')}" style="color: #4a90e2; text-decoration: none; cursor: pointer;">${sessionId.slice(0, 8)}...</a>`;
          // Add click handler
          const link = blameDetailTrace.querySelector('.blame-session-link');
          if (link) {
            link.addEventListener('click', (e) => {
              e.preventDefault();
              const sid = link.getAttribute('data-session');
              navigateToTranscript(sid).catch(console.error);
            });
          }
        } else {
          blameDetailTrace.textContent = `Session: —`;
        }
      }

      // Show details panel
      if (blameDetailsPanel) {
        blameDetailsPanel.classList.add('is-visible');
      }
    });

    blameCodeBlock.appendChild(row);
  });
}

async function loadProjectFilesForBlame() {
  // Clear search box when loading new project
  if (fileSearchInput) {
    fileSearchInput.value = '';
  }

  const projectDir = getProjectDir();
  if (!projectDir) {
    if (blameFileList) {
      blameFileList.textContent = '';
      const li = document.createElement('li');
      li.className = 'muted';
      li.textContent = 'Select a project first.';
      blameFileList.appendChild(li);
    }
    return;
  }

  const invoke = window.__TAURI__?.invoke;
  if (!invoke) {
    if (blameFileList) {
      blameFileList.textContent = '';
      const li = document.createElement('li');
      li.className = 'muted';
      li.textContent = 'Blame requires the desktop app (Tauri).';
      blameFileList.appendChild(li);
    }
    return;
  }

  setStatus('Loading files…');

  try {
    // Load all project files
    const filesRes = await invoke('list_project_files', { projectDir });
    allFiles = filesRes?.files ?? [];

    // Load agent-touched files
    agentTouchedFiles.clear();
    try {
      const agentRes = await invoke('list_agent_touched_files', { projectDir });
      (agentRes?.files ?? []).forEach((f) => agentTouchedFiles.add(f));
    } catch (err) {
      // If the command fails (e.g., no traces yet), just continue with empty set
      console.debug('Could not load agent-touched files:', err);
    }

    if (allFiles.length === 0) {
      setStatus('No files found (or folder is empty).');
    } else {
      const agentCount = agentTouchedFiles.size;
      setStatus(`Loaded ${allFiles.length} files (${agentCount} touched by agents).`);
    }

    renderFileList();
  } catch (err) {
    console.error(err);
    setStatus('Failed to load files. See console for details.');
  }
}

async function tauriPickProjectDirectory() {
  const tauri = window.__TAURI__;
  const open = tauri?.dialog?.open;
  if (!open) {
    return null;
  }

  const picked = await open({
    directory: true,
    multiple: false,
    title: 'Select project directory',
  });

  // Tauri returns `string | string[] | null`
  if (Array.isArray(picked)) {
    return picked[0] ?? null;
  }
  return picked ?? null;
}

function closeProjectPicker() {
  if (!picker) return;
  picker.classList.remove('is-open');
  picker.setAttribute('aria-hidden', 'true');
}

function loadRecentProjects() {
  try {
    const stored = localStorage.getItem(RECENT_PROJECTS_KEY);
    if (!stored) return DEFAULT_PROJECTS;
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return DEFAULT_PROJECTS;
    const reduced = parsed.reduce((acc, p) => {
      if (p?.name && p?.path) {
        acc.push({ name: p.name, path: p.path, tag: p.tag || 'Recent' });
      }
      return acc;
    }, []);
    return reduced.length > 0 ? reduced : DEFAULT_PROJECTS;
  } catch {
    return DEFAULT_PROJECTS;
  }
}

function persistRecentProject(project) {
  if (!project?.path) return;
  const existing = loadRecentProjects();
  const next = [project, ...existing.filter((p) => p.path !== project.path)].slice(
    0,
    MAX_RECENT_PROJECTS
  );
  try {
    localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(next));
    pickerProjects = next;
  } catch {
    pickerProjects = next;
  }
}

pickerProjects = loadRecentProjects();

function loadHomeDirectory() {
  try {
    return localStorage.getItem(HOME_DIRECTORY_KEY) || '';
  } catch {
    return '';
  }
}

function saveHomeDirectory(homeDir) {
  try {
    if (homeDir && homeDir.trim()) {
      localStorage.setItem(HOME_DIRECTORY_KEY, homeDir.trim());
    } else {
      localStorage.removeItem(HOME_DIRECTORY_KEY);
    }
  } catch (e) {
    console.error('Failed to save home directory:', e);
  }
}

function getHomeDirectory() {
  return homeDirectoryInput?.value?.trim() || '';
}

function refreshBlameIfVisible() {
  const currentView = document.querySelector('.view.is-visible')?.dataset?.view;
  if (currentView === 'blame') {
    loadProjectFilesForBlame().catch((err) => console.error(err));
  }
}

function applyProjectSelection(path, label) {
  if (!path) return;
  setProject(path);
  setStatus(label ? `Project set: ${label}` : `Project set: ${path}`);
  const existingTag = pickerProjects.find((p) => p.path === path)?.tag || 'Recent';
  persistRecentProject({ name: label || path, path, tag: existingTag });
  closeProjectPicker();

  refreshBlameIfVisible();
}

function renderPickerList(query) {
  if (!pickerList) return;
  pickerList.textContent = '';

  const normalized = (query || '').trim().toLowerCase();
  const filtered =
    normalized.length === 0
      ? pickerProjects
      : pickerProjects.filter(
          (p) =>
            p.name.toLowerCase().includes(normalized) ||
            p.path.toLowerCase().includes(normalized) ||
            p.tag.toLowerCase().includes(normalized)
        );

  if (filtered.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'muted';
    empty.textContent = 'No matches. Try a different search or paste a path below.';
    pickerList.appendChild(empty);
    return;
  }

  filtered.forEach((project) => {
    const card = document.createElement('div');
    card.className = 'picker-card';

    const title = document.createElement('h4');
    title.textContent = project.name;

    const path = document.createElement('p');
    path.className = 'muted';
    path.textContent = project.path;

    const tag = document.createElement('span');
    tag.className = 'tag';
    tag.textContent = project.tag;

    card.appendChild(title);
    card.appendChild(path);
    card.appendChild(tag);

    card.addEventListener('click', () => applyProjectSelection(project.path, project.name));
    pickerList.appendChild(card);
  });
}

function openProjectPicker() {
  if (!picker) return;
  pickerProjects = loadRecentProjects();
  picker.classList.add('is-open');
  picker.setAttribute('aria-hidden', 'false');
  renderPickerList('');
  if (pickerManual) {
    pickerManual.value = '';
  }
  setTimeout(() => pickerSearch?.focus(), 0);
}

function wireCheckboxes() {
  if (agentTouchedOnlyCheckbox) {
    agentTouchedOnlyCheckbox.addEventListener('change', () => {
      renderFileList();
    });
  }
  if (fileSearchInput) {
    fileSearchInput.addEventListener('input', () => {
      renderFileList();
    });
  }
}

function wireDetailsPanel() {
  if (closeDetailsBtn) {
    closeDetailsBtn.addEventListener('click', () => {
      if (blameDetailsPanel) {
        blameDetailsPanel.classList.remove('is-visible');
      }
      // Deselect the line
      document
        .querySelectorAll('.code-line.is-selected')
        .forEach((el) => el.classList.remove('is-selected'));
    });
  }
}
function wireActions() {
  const actionButtons = document.querySelectorAll('[data-action]');

  actionButtons.forEach((button) => {
    button.addEventListener('click', async () => {
      const action = button.dataset.action;

      try {
        switch (action) {
          case 'configure-rules':
            navigate('settings');
            setStatus('Opened settings.');
            break;
          case 'open-project': {
            setStatus('Opening in-app project picker…');
            openProjectPicker();
            break;
          }
          case 'refresh-traces':
          case 'scan-traces': {
            const invoke = window.__TAURI__?.invoke;
            if (invoke) {
              setStatus('Scanning traces…');
              const projectDir = projectPathInput?.value?.trim() || null;
              const res = await invoke('scan_traces', { projectDir });
              if (res?.trace_dir) {
                setStatus(
                  `Traces: ${res.trace_count} · Trace dir: ${res.trace_dir}`
                );
              } else {
                setStatus(`Traces: ${res?.trace_count ?? 0}`);
              }
            } else {
              setStatus('Scan/Refresh: demo only in browser preview. See ui/TODO.md.');
            }
            break;
          }
          case 'export-html':
            setStatus('Export HTML: not implemented yet. See ui/TODO.md.');
            break;
          case 'refresh-transcripts':
            await loadTranscriptsList();
            break;
          case 'picker-system': {
            setStatus('Opening system picker…');
            const picked = await tauriPickProjectDirectory();
            if (picked) {
              applyProjectSelection(picked);
            } else {
              setStatus('System picker was cancelled or unavailable.');
            }
            break;
          }
          case 'picker-apply': {
            const manualPath = pickerManual?.value?.trim();
            if (manualPath) {
              applyProjectSelection(manualPath);
            } else {
              setStatus('Enter a path to continue.');
            }
            break;
          }
          case 'save-config':
            const homeDir = getHomeDirectory();
            saveHomeDirectory(homeDir);
            setStatus(homeDir ? `Settings saved. Home directory: ${homeDir}` : 'Settings saved. Using system home directory.');
            break;
          case 'add-rule':
            setStatus('Add Rule: not implemented yet. See ui/TODO.md.');
            break;
          default:
            setStatus(`Clicked: ${action ?? 'unknown action'}`);
        }
      } catch (err) {
        console.error(err);
        setStatus('Action failed. See console for details.');
      }
    });
  });
}

pickerClose?.addEventListener('click', closeProjectPicker);
pickerSearch?.addEventListener('input', (event) => renderPickerList(event.target.value));

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && picker?.classList.contains('is-open')) {
    closeProjectPicker();
  }
});

// Timeline functions
async function loadTimeline() {
  const timelineBody = document.getElementById('timeline-body');
  if (!timelineBody) return;

  const invoke = window.__TAURI__?.invoke;
  if (!invoke) {
    timelineBody.innerHTML = '<tr><td colspan="5" class="muted">Timeline requires the desktop app (Tauri).</td></tr>';
    return;
  }

  setStatus('Loading timeline…');
  timelineBody.innerHTML = '<tr><td colspan="5" class="muted">Loading…</td></tr>';

  try {
    const projectDir = getProjectDir();
    if (!projectDir) {
      timelineBody.innerHTML = '<tr><td colspan="5" class="muted">Select a project first.</td></tr>';
      return;
    }

    const limit = parseInt(document.getElementById('timeline-limit')?.value ?? '50', 10);
    const skip_codex = document.getElementById('timeline-filter-codex')?.checked ?? false;

    const result = await invoke('list_timeline', {
      project_dir: projectDir,
      limit,
      skip_codex
    });

    if (!result || !result.events) {
      timelineBody.innerHTML = '<tr><td colspan="5" class="muted">No timeline data found.</td></tr>';
      setStatus('No timeline data.');
      return;
    }

    const events = result.events;

    if (events.length === 0) {
      timelineBody.innerHTML = '<tr><td colspan="5" class="muted">No timeline data found.</td></tr>';
      setStatus('No timeline data.');
      return;
    }

    timelineBody.textContent = '';
    events.forEach((event) => {
      const tr = document.createElement('tr');

      const timestamp = event.timestamp ? new Date(event.timestamp).toLocaleString() : '—';
      const action = event.action ?? '—';
      const file_path = event.file_path ?? '—';
      const model = event.model ?? '—';
      const agent = event.agent_tool ?
        (event.agent_version ? `${event.agent_tool}@${event.agent_version}` : event.agent_tool)
        : '—';

      // Make file path clickable to navigate to blame viewer
      let fileCell = file_path;
      if (file_path !== '—') {
        fileCell = `<a href="#" class="timeline-file-link" data-file="${file_path.replace(/"/g, '&quot;')}" style="color: #4a90e2; text-decoration: none; cursor: pointer;">${file_path}</a>`;
      }

      tr.innerHTML = `
        <td>${timestamp}</td>
        <td>${action}</td>
        <td style="max-width: 300px; overflow: hidden; text-overflow: ellipsis;">${fileCell}</td>
        <td>${model}</td>
        <td>${agent}</td>
      `;
      timelineBody.appendChild(tr);
    });

    // Wire up timeline file link clicks
    document.querySelectorAll('.timeline-file-link').forEach((link) => {
      link.addEventListener('click', (e) => {
        e.preventDefault();
        const filePath = link.getAttribute('data-file');
        navigateToBlamFile(filePath).catch(console.error);
      });
    });

    setStatus(`Timeline: ${events.length} edits (${result.total_count} total)`);
  } catch (err) {
    console.error('Timeline error:', err);
    setStatus('Failed to load timeline.');
    timelineBody.innerHTML = `<tr><td colspan="5" class="muted">Error: ${err.message || err}</td></tr>`;
  }
}

// Wire timeline controls
document.getElementById('timeline-limit')?.addEventListener('change', () => {
  loadTimeline().catch(console.error);
});
document.getElementById('timeline-filter-codex')?.addEventListener('change', () => {
  loadTimeline().catch(console.error);
});

// Make project name clickable to open picker
document.getElementById('project-name')?.addEventListener('click', () => {
  openProjectPicker();
});

navItems.forEach((button) => {
  button.addEventListener('click', () => {
    const view = button.dataset.view;
    navItems.forEach((item) => item.classList.remove('is-active'));
    button.classList.add('is-active');

    views.forEach((panel) => {
      panel.classList.toggle('is-visible', panel.dataset.view === view);
    });

    if (viewTitle) {
      viewTitle.textContent = titleMap[view] ?? 'AI Blame';
    }

    if (view === 'blame') {
      loadProjectFilesForBlame().catch((err) => console.error(err));
    }

    if (view === 'timeline') {
      loadTimeline().catch((err) => console.error(err));
    }

    if (view === 'transcripts') {
      loadTranscriptsList().catch((err) => console.error(err));
    }

    if (view === 'settings') {
      // Load home directory from localStorage when entering settings
      const savedHomeDir = loadHomeDirectory();
      if (homeDirectoryInput) {
        homeDirectoryInput.value = savedHomeDir;
      }
    }
  });
});

// Auto-open project picker on startup if no project is set
window.addEventListener('DOMContentLoaded', () => {
  const projectDir = getProjectDir();
  if (!projectDir || projectDir === '') {
    setTimeout(() => openProjectPicker(), 100);
  }
});

wireCheckboxes();
wireDetailsPanel();
wireActions();
wireTranscriptSearch();

// Transcript Viewer Functions
async function loadTranscriptsList() {
  if (!transcriptList) return;
  transcriptList.textContent = '';

  const invoke = window.__TAURI__?.invoke;
  if (!invoke) {
    const li = document.createElement('li');
    li.className = 'muted';
    li.textContent = 'Transcripts require the desktop app (Tauri).';
    transcriptList.appendChild(li);
    return;
  }

  setStatus('Loading transcripts…');

  try {
    const projectDir = getProjectDir();
    const res = await invoke('list_transcripts', { projectDir, limit: 100 });
    transcriptsList = res?.transcripts ?? [];

    if (transcriptsList.length === 0) {
      const li = document.createElement('li');
      li.className = 'muted';
      li.textContent = 'No transcripts found.';
      transcriptList.appendChild(li);
      setStatus('No transcripts found.');
      return;
    }

    setStatus(`Loaded ${transcriptsList.length} transcript${transcriptsList.length === 1 ? '' : 's'}.`);
    renderTranscriptList();
  } catch (err) {
    console.error(err);
    setStatus('Failed to load transcripts. See console for details.');
    const li = document.createElement('li');
    li.className = 'muted';
    li.textContent = 'Failed to load transcripts.';
    transcriptList.appendChild(li);
  }
}

function renderTranscriptList() {
  if (!transcriptList) return;
  transcriptList.textContent = '';

  // Use search results if available, otherwise use regular list
  const items = searchResults
    ? searchResults.matching_transcripts.map(m => ({ ...m.transcript, snippets: m.matches }))
    : transcriptsList.map(t => ({ ...t, snippets: [] }));

  items.forEach((t) => {
    const li = document.createElement('li');
    li.className = 'transcript-item';
    // Add keyboard accessibility
    li.tabIndex = 0;
    li.setAttribute('role', 'button');
    li.setAttribute('aria-label', `Open transcript: ${t.slug || safeTruncate(t.session_id, 20)}`);
    li.setAttribute('data-session', t.session_id);  // Add data attribute for programmatic selection

    const titleEl = document.createElement('div');
    titleEl.className = 'transcript-item-title';
    titleEl.textContent = t.slug || safeTruncate(t.session_id, 12);

    const metaEl = document.createElement('div');
    metaEl.className = 'transcript-item-meta';
    const date = new Date(t.start_time);
    metaEl.textContent = `${date.toLocaleDateString()} · ${t.message_count} msgs`;

    const tagEl = document.createElement('span');
    tagEl.className = 'tag';
    tagEl.textContent = t.agent_tool;

    li.appendChild(titleEl);
    li.appendChild(metaEl);
    li.appendChild(tagEl);

    // Show snippets if available (from search results)
    if (t.snippets && t.snippets.length > 0) {
      const snippetsEl = document.createElement('div');
      snippetsEl.className = 'transcript-item-snippets';

      // Show up to 2 snippets
      t.snippets.slice(0, 2).forEach((snippet) => {
        const snippetEl = document.createElement('div');
        snippetEl.className = 'transcript-item-snippet';

        const typeEl = document.createElement('span');
        typeEl.className = 'transcript-item-snippet-type';
        typeEl.textContent = snippet.block_type;

        snippetEl.appendChild(typeEl);
        snippetEl.appendChild(document.createTextNode(safeTruncate(snippet.snippet, 80)));
        snippetsEl.appendChild(snippetEl);
      });

      li.appendChild(snippetsEl);
    }

    // Click handler
    const selectTranscript = async () => {
      transcriptList.querySelectorAll('li.active').forEach((el) => el.classList.remove('active'));
      li.classList.add('active');
      await loadTranscript(t.session_id);
    };

    li.addEventListener('click', selectTranscript);

    // Keyboard handler for accessibility
    li.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        selectTranscript();
      }
    });

    transcriptList.appendChild(li);
  });
}

async function loadTranscript(sessionId) {
  const invoke = window.__TAURI__?.invoke;
  if (!invoke) return;

  setStatus(`Loading transcript: ${sessionId}…`);

  try {
    const projectDir = getProjectDir();
    const transcript = await invoke('get_transcript', { sessionOrPath: sessionId, projectDir });
    currentTranscript = transcript;

    // If there's an active cross-session search, use it for in-transcript filtering + highlighting
    if (crossSessionSearchQuery && crossSessionSearchQuery.trim().length >= 2) {
      transcriptSearchQuery = crossSessionSearchQuery;
      if (transcriptSearch) {
        transcriptSearch.value = crossSessionSearchQuery;
      }
      renderTranscriptFiltered(transcript, transcriptSearchQuery);
    } else {
      // Reset search when loading a new transcript without active search
      transcriptSearchQuery = '';
      if (transcriptSearch) {
        transcriptSearch.value = '';
      }
      renderTranscriptFiltered(transcript, '');
    }

    setStatus(`Viewing transcript: ${transcript.meta.slug || sessionId}`);
  } catch (err) {
    console.error(err);
    setStatus('Failed to load transcript. See console for details.');
  }
}

function renderTranscript(transcript) {
  // Update meta
  if (transcriptMetaSession) {
    transcriptMetaSession.textContent = transcript.meta.slug || transcript.meta.session_id;
  }
  if (transcriptMetaAgent) {
    let agentStr = transcript.meta.agent_tool;
    if (transcript.meta.agent_version) {
      agentStr += `@${transcript.meta.agent_version}`;
    }
    transcriptMetaAgent.textContent = agentStr;
  }
  if (transcriptMetaTime) {
    const start = new Date(transcript.meta.start_time);
    let timeStr = start.toLocaleString();
    if (transcript.meta.end_time) {
      const end = new Date(transcript.meta.end_time);
      const durationMs = end - start;
      const durationMins = Math.round(durationMs / 60000);
      timeStr += ` (${durationMins} min)`;
    }
    transcriptMetaTime.textContent = timeStr;
  }

  // Render messages
  if (!transcriptMessages) return;
  transcriptMessages.textContent = '';

  if (!transcript.messages || transcript.messages.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'transcript-empty';
    empty.innerHTML = '<p class="muted">No messages in this transcript.</p>';
    transcriptMessages.appendChild(empty);
    return;
  }

  transcript.messages.forEach((msg) => {
    const msgEl = document.createElement('div');
    msgEl.className = `transcript-message transcript-message-${msg.role}`;

    const headerEl = document.createElement('div');
    headerEl.className = 'transcript-message-header';

    const roleEl = document.createElement('span');
    roleEl.className = 'transcript-role';
    roleEl.textContent = msg.role.charAt(0).toUpperCase() + msg.role.slice(1);
    headerEl.appendChild(roleEl);

    if (msg.model) {
      const modelEl = document.createElement('span');
      modelEl.className = 'transcript-model';
      modelEl.textContent = msg.model;
      headerEl.appendChild(modelEl);
    }

    const timeEl = document.createElement('span');
    timeEl.className = 'transcript-time';
    const msgTime = new Date(msg.timestamp);
    timeEl.textContent = msgTime.toLocaleTimeString();
    headerEl.appendChild(timeEl);

    msgEl.appendChild(headerEl);

    // Render content blocks
    msg.content.forEach((block) => {
      const blockEl = document.createElement('div');
      blockEl.className = 'transcript-content-block';

      switch (block.type) {
        case 'text':
          blockEl.className += ' transcript-text';
          blockEl.textContent = block.text;
          break;
        case 'thinking':
          blockEl.className += ' transcript-thinking';
          const thinkingLabel = document.createElement('div');
          thinkingLabel.className = 'transcript-block-label';
          thinkingLabel.textContent = 'Thinking';
          blockEl.appendChild(thinkingLabel);
          const thinkingContent = document.createElement('div');
          thinkingContent.textContent = safeTruncate(block.thinking, 500);
          blockEl.appendChild(thinkingContent);
          break;
        case 'tool_use':
          blockEl.className += ' transcript-tool-use';
          const toolLabel = document.createElement('div');
          toolLabel.className = 'transcript-block-label';
          toolLabel.textContent = `Tool: ${block.name}`;
          blockEl.appendChild(toolLabel);
          if (block.input && Object.keys(block.input).length > 0) {
            const inputEl = document.createElement('pre');
            inputEl.className = 'transcript-tool-input';
            const inputStr = JSON.stringify(block.input, null, 2);
            inputEl.textContent = safeTruncate(inputStr, 300);
            blockEl.appendChild(inputEl);
          }
          break;
        case 'tool_result':
          blockEl.className += ' transcript-tool-result';
          if (block.is_error) blockEl.classList.add('is-error');
          const resultLabel = document.createElement('div');
          resultLabel.className = 'transcript-block-label';
          resultLabel.textContent = block.is_error ? 'Error' : 'Result';
          blockEl.appendChild(resultLabel);
          const resultContent = document.createElement('pre');
          resultContent.textContent = safeTruncate(block.content, 300);
          blockEl.appendChild(resultContent);
          break;
        case 'file_operation':
          blockEl.className += ' transcript-file-op';
          const fileLabel = document.createElement('div');
          fileLabel.className = 'transcript-block-label';
          fileLabel.textContent = `File ${block.operation}: ${block.file_path}`;
          blockEl.appendChild(fileLabel);
          break;
        case 'command':
          blockEl.className += ' transcript-command';
          const cmdLabel = document.createElement('div');
          cmdLabel.className = 'transcript-block-label';
          cmdLabel.textContent = 'Command';
          blockEl.appendChild(cmdLabel);
          const cmdContent = document.createElement('pre');
          cmdContent.textContent = block.command;
          blockEl.appendChild(cmdContent);
          if (block.output) {
            const outputEl = document.createElement('pre');
            outputEl.className = 'transcript-command-output';
            outputEl.textContent = safeTruncate(block.output, 300);
            blockEl.appendChild(outputEl);
          }
          break;
        case 'code':
          blockEl.className += ' transcript-code';
          const codeEl = document.createElement('pre');
          codeEl.textContent = safeTruncate(block.code, 500);
          blockEl.appendChild(codeEl);
          break;
        default:
          blockEl.className += ' transcript-unknown';
          blockEl.textContent = JSON.stringify(block);
      }

      msgEl.appendChild(blockEl);
    });

    transcriptMessages.appendChild(msgEl);
  });
}

// Helper function to check if a content block matches the search query
function blockMatchesQuery(block, query) {
  if (!query) return true;
  const lowerQuery = query.toLowerCase();

  switch (block.type) {
    case 'text':
      return (block.text || '').toLowerCase().includes(lowerQuery);
    case 'thinking':
      return (block.thinking || '').toLowerCase().includes(lowerQuery);
    case 'tool_use':
      return (
        (block.name || '').toLowerCase().includes(lowerQuery) ||
        JSON.stringify(block.input || {}).toLowerCase().includes(lowerQuery)
      );
    case 'tool_result':
      return (block.content || '').toLowerCase().includes(lowerQuery);
    case 'command':
      return (
        (block.command || '').toLowerCase().includes(lowerQuery) ||
        (block.output || '').toLowerCase().includes(lowerQuery)
      );
    case 'code':
      return (block.code || '').toLowerCase().includes(lowerQuery);
    case 'file_operation':
      return (block.file_path || '').toLowerCase().includes(lowerQuery);
    default:
      return JSON.stringify(block).toLowerCase().includes(lowerQuery);
  }
}

// Helper function to check if a message matches the search query
function messageMatchesQuery(message, query) {
  if (!query) return true;
  const lowerQuery = query.toLowerCase();

  // Check message metadata
  if ((message.role || '').toLowerCase().includes(lowerQuery) || (message.model || '').toLowerCase().includes(lowerQuery)) {
    return true;
  }

  // Check message content blocks
  return (message.content || []).some((block) => blockMatchesQuery(block, query));
}

// Render transcript with optional search filtering
function renderTranscriptFiltered(transcript, query) {
  if (!transcriptMessages) return;
  transcriptMessages.textContent = '';

  if (!transcript.messages || transcript.messages.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'transcript-empty';
    empty.innerHTML = '<p class="muted">No messages in this transcript.</p>';
    transcriptMessages.appendChild(empty);
    return;
  }

  // Filter messages if search query is present
  const filteredMessages = query
    ? transcript.messages.filter((msg) => messageMatchesQuery(msg, query))
    : transcript.messages;

  // If no messages match, show empty state
  if (filteredMessages.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'transcript-empty';
    empty.innerHTML = `<p class="muted">No messages match "${query}".</p>`;
    transcriptMessages.appendChild(empty);
    return;
  }

  // Render filtered messages
  filteredMessages.forEach((msg) => {
    const msgEl = document.createElement('div');
    msgEl.className = `transcript-message transcript-message-${msg.role}`;

    const headerEl = document.createElement('div');
    headerEl.className = 'transcript-message-header';

    const roleEl = document.createElement('span');
    roleEl.className = 'transcript-role';
    roleEl.textContent = msg.role.charAt(0).toUpperCase() + msg.role.slice(1);
    headerEl.appendChild(roleEl);

    if (msg.model) {
      const modelEl = document.createElement('span');
      modelEl.className = 'transcript-model';
      modelEl.textContent = msg.model;
      headerEl.appendChild(modelEl);
    }

    const timeEl = document.createElement('span');
    timeEl.className = 'transcript-time';
    const msgTime = new Date(msg.timestamp);
    timeEl.textContent = msgTime.toLocaleTimeString();
    headerEl.appendChild(timeEl);

    msgEl.appendChild(headerEl);

    // Render content blocks (all blocks, but we know at least one matches if query exists)
    msg.content.forEach((block) => {
      const blockEl = document.createElement('div');
      blockEl.className = 'transcript-content-block';

      switch (block.type) {
        case 'text':
          blockEl.className += ' transcript-text';
          // Use innerHTML with highlighting if there's a query
          blockEl.innerHTML = highlightMatches(block.text, query);
          break;
        case 'thinking':
          blockEl.className += ' transcript-thinking';
          const thinkingLabel = document.createElement('div');
          thinkingLabel.className = 'transcript-block-label';
          thinkingLabel.textContent = 'Thinking';
          blockEl.appendChild(thinkingLabel);
          const thinkingContent = document.createElement('div');
          thinkingContent.innerHTML = highlightMatches(safeTruncate(block.thinking, 500), query);
          blockEl.appendChild(thinkingContent);
          break;
        case 'tool_use':
          blockEl.className += ' transcript-tool-use';
          const toolLabel = document.createElement('div');
          toolLabel.className = 'transcript-block-label';
          toolLabel.innerHTML = highlightMatches(`Tool: ${block.name}`, query);
          blockEl.appendChild(toolLabel);
          if (block.input && Object.keys(block.input).length > 0) {
            const inputEl = document.createElement('pre');
            inputEl.className = 'transcript-tool-input';
            const inputStr = JSON.stringify(block.input, null, 2);
            inputEl.innerHTML = highlightMatches(safeTruncate(inputStr, 300), query);
            blockEl.appendChild(inputEl);
          }
          break;
        case 'tool_result':
          blockEl.className += ' transcript-tool-result';
          if (block.is_error) blockEl.classList.add('is-error');
          const resultLabel = document.createElement('div');
          resultLabel.className = 'transcript-block-label';
          resultLabel.textContent = block.is_error ? 'Error' : 'Result';
          blockEl.appendChild(resultLabel);
          const resultContent = document.createElement('pre');
          resultContent.innerHTML = highlightMatches(safeTruncate(block.content, 300), query);
          blockEl.appendChild(resultContent);
          break;
        case 'file_operation':
          blockEl.className += ' transcript-file-op';
          const fileLabel = document.createElement('div');
          fileLabel.className = 'transcript-block-label';
          fileLabel.innerHTML = highlightMatches(`File ${block.operation}: ${block.file_path}`, query);
          blockEl.appendChild(fileLabel);
          break;
        case 'command':
          blockEl.className += ' transcript-command';
          const cmdLabel = document.createElement('div');
          cmdLabel.className = 'transcript-block-label';
          cmdLabel.textContent = 'Command';
          blockEl.appendChild(cmdLabel);
          const cmdContent = document.createElement('pre');
          cmdContent.innerHTML = highlightMatches(block.command, query);
          blockEl.appendChild(cmdContent);
          if (block.output) {
            const outputEl = document.createElement('pre');
            outputEl.className = 'transcript-command-output';
            outputEl.innerHTML = highlightMatches(safeTruncate(block.output, 300), query);
            blockEl.appendChild(outputEl);
          }
          break;
        case 'code':
          blockEl.className += ' transcript-code';
          const codeEl = document.createElement('pre');
          codeEl.innerHTML = highlightMatches(safeTruncate(block.code, 500), query);
          blockEl.appendChild(codeEl);
          break;
        default:
          blockEl.className += ' transcript-unknown';
          blockEl.innerHTML = highlightMatches(JSON.stringify(block), query);
      }

      msgEl.appendChild(blockEl);
    });

    transcriptMessages.appendChild(msgEl);
  });
}

// Debounce helper
function debounce(fn, delay) {
  let timeout;
  return (...args) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => fn(...args), delay);
  };
}

// Perform cross-session search
async function performCrossSessionSearch(query) {
  const invoke = window.__TAURI__?.invoke;
  if (!invoke) return;

  if (!query || query.trim().length < 2) {
    // Clear search results and show all transcripts
    searchResults = null;
    if (transcriptSearchInfo) {
      transcriptSearchInfo.textContent = '';
    }
    renderTranscriptList();
    return;
  }

  setStatus(`Searching transcripts for "${query}"…`);

  try {
    const projectDir = getProjectDir();
    const result = await invoke('search_transcripts', {
      projectDir,
      query: query.trim(),
      limit: 50,
    });

    searchResults = result;

    if (transcriptSearchInfo) {
      transcriptSearchInfo.textContent = `Found ${result.total_matches} matching transcript${result.total_matches === 1 ? '' : 's'}`;
    }

    renderTranscriptList();
    setStatus(`Found ${result.total_matches} transcript${result.total_matches === 1 ? '' : 's'} matching "${query}"`);
  } catch (err) {
    console.error('Search failed:', err);
    setStatus('Search failed. See console for details.');
  }
}

// Wire up transcript search (both cross-session and in-transcript)
function wireTranscriptSearch() {
  // Cross-session search (in the transcript list)
  if (transcriptListSearch) {
    const debouncedSearch = debounce(performCrossSessionSearch, 300);

    transcriptListSearch.addEventListener('input', (e) => {
      crossSessionSearchQuery = e.target.value;
      debouncedSearch(crossSessionSearchQuery);
    });

    // Also sync to in-transcript search when loading
    transcriptListSearch.addEventListener('change', () => {
      // When cross-session search changes, also update in-transcript filter
      if (transcriptSearch && crossSessionSearchQuery) {
        transcriptSearch.value = crossSessionSearchQuery;
        transcriptSearchQuery = crossSessionSearchQuery;
        if (currentTranscript) {
          renderTranscriptFiltered(currentTranscript, transcriptSearchQuery);
        }
      }
    });
  }

  // In-transcript search (within the loaded transcript)
  if (transcriptSearch) {
    transcriptSearch.addEventListener('input', (e) => {
      transcriptSearchQuery = e.target.value;
      if (currentTranscript) {
        renderTranscriptFiltered(currentTranscript, transcriptSearchQuery);
      }
    });
  }
}
