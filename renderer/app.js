const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;
const currentWindow = window.__TAURI__.window.getCurrentWindow();
const isMacPlatform = /\bMac\b/.test(navigator.userAgent);
const SHARED_DATA_RECONCILE_INTERVAL = 5 * 60 * 1000;
const beforeQuitHooks = [];
const saveHooks = [];
let quitInProgress = false;
let currentAppNotice = null;
let noticeTimer = null;
let reconcileTimer = null;
let currentActiveTab = 'tasks';

window.currentStorageStatus = null;
window.appMetadata = null;

function debounce(fn, ms) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

window.debounce = debounce;

async function callCommand(command, args = {}) {
  const result = await invoke(command, args);
  if (!result.success) {
    throw result.error || { code: 'unknown', message: 'Unknown error.' };
  }
  return result.data;
}

window.callCommand = callCommand;
window.registerBeforeQuitHook = function registerBeforeQuitHook(fn) {
  beforeQuitHooks.push(fn);
};

window.registerSaveHook = function registerSaveHook(fn) {
  saveHooks.push(fn);
};

window.flushPendingAppState = async function flushPendingAppState() {
  for (const hook of beforeQuitHooks) {
    await hook();
  }

  await Promise.all(saveHooks.map((hook) => hook()));
};

function updateTabCount(tabName, count) {
  const el = document.getElementById(`tab-count-${tabName}`);
  if (el) el.textContent = count > 0 ? `(${count})` : '';
}

window.updateTabCount = updateTabCount;

function renderTopBanner() {
  const banner = document.getElementById('storage-banner');
  banner.classList.remove('info', 'warning', 'danger');

  if (window.currentStorageStatus && (window.currentStorageStatus.mode === 'syncUnavailable' || window.currentStorageStatus.mode === 'localUnavailable')) {
    banner.textContent = window.currentStorageStatus.message || 'Shared data is currently unavailable.';
    banner.classList.add('danger');
    banner.classList.remove('hidden');
    return;
  }

  if (currentAppNotice && currentAppNotice.message) {
    banner.textContent = currentAppNotice.message;
    banner.classList.add(currentAppNotice.tone || 'info');
    banner.classList.remove('hidden');
    if (currentAppNotice.onClick) {
      banner.style.cursor = 'pointer';
      banner.onclick = currentAppNotice.onClick;
    } else {
      banner.style.cursor = '';
      banner.onclick = null;
    }
    return;
  }

  banner.textContent = '';
  banner.classList.add('hidden');
  banner.style.cursor = '';
  banner.onclick = null;
}

window.showAppNotice = function showAppNotice(message, tone = 'info', timeoutMs = 7000, onClick = null) {
  currentAppNotice = { message, tone, onClick };
  if (noticeTimer) clearTimeout(noticeTimer);
  if (timeoutMs > 0) {
    noticeTimer = setTimeout(() => {
      currentAppNotice = null;
      renderTopBanner();
    }, timeoutMs);
  }
  renderTopBanner();
};

window.clearAppNotice = function clearAppNotice() {
  currentAppNotice = null;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = null;
  renderTopBanner();
};

function setActiveTab(tabName, section) {
  currentActiveTab = tabName;
  document.querySelectorAll('.tab').forEach((tab) => {
    const active = tab.dataset.tab === tabName;
    tab.classList.toggle('active', active);
    tab.setAttribute('aria-selected', active ? 'true' : 'false');
  });

  document.querySelectorAll('.tab-content').forEach((panel) => {
    panel.classList.toggle('active', panel.id === `tab-${tabName}`);
  });

  const activePanel = document.getElementById(`tab-${tabName}`);
  if (activePanel && window.syncCompact) {
    activePanel.querySelectorAll('.kanban-board, #tickets-list').forEach(window.syncCompact);
  }

  if (section) {
    const target = document.getElementById(`${section}-section`);
    if (target) target.scrollIntoView({ block: 'start', behavior: 'smooth' });
  }
}

window.applyColorTheme = function applyColorTheme(theme) {
  if (theme === 'light' || theme === 'dark') {
    document.documentElement.dataset.theme = theme;
  } else {
    delete document.documentElement.dataset.theme;
  }
};

window.syncCompact = function syncCompact(el) {
  if (!el) return;
  el.classList.remove('compact');
  if (el.scrollHeight > el.clientHeight) el.classList.add('compact');
};

window.setActiveTab = setActiveTab;
window.getActiveTabName = function getActiveTabName() {
  return currentActiveTab;
};

window.applyStandingColumnVisibility = function applyStandingColumnVisibility(visible) {
  document.querySelectorAll('.kanban-section[data-column="standing"]').forEach((section) => {
    section.classList.toggle('hidden', !visible);
  });
  document.querySelectorAll('option[value="standing"]').forEach((opt) => {
    opt.hidden = !visible;
    if (!visible && opt.selected) {
      opt.closest('select').value = 'todo';
    }
  });
};

window.applyPersonalTabVisibility = function applyPersonalTabVisibility(visible) {
  const personalTab = document.querySelector('.tab[data-tab="personal"]');
  const personalPanel = document.getElementById('tab-personal');
  if (!personalTab || !personalPanel) return;

  personalTab.classList.toggle('hidden', !visible);
  personalPanel.classList.toggle('hidden', !visible);

  if (!visible && currentActiveTab === 'personal') {
    setActiveTab('tasks');
  }
};

function renderStorageBanner(status) {
  window.currentStorageStatus = status;
  renderTopBanner();
  window.dispatchEvent(new CustomEvent('storage-status-changed', { detail: status }));
}

window.refreshStorageStatus = async function refreshStorageStatus() {
  try {
    const status = await callCommand('get_storage_status');
    renderStorageBanner(status);
    return status;
  } catch (error) {
    renderStorageBanner({
      mode: 'localUnavailable',
      configuredPath: null,
      activePath: null,
      sharedDataAvailable: false,
      message: error.message || 'Storage status is unavailable.',
    });
    return window.currentStorageStatus;
  }
};

window.refreshAppMetadata = async function refreshAppMetadata() {
  try {
    const metadata = await callCommand('get_app_metadata');
    window.appMetadata = metadata;
    window.dispatchEvent(new CustomEvent('app-metadata-changed', { detail: metadata }));
    return metadata;
  } catch (error) {
    console.error('Failed to load app metadata:', error);
    return null;
  }
};

window.openExternal = async function openExternal(url) {
  return callCommand('open_external_url', { url });
};

function startSharedDataReconcile() {
  if (reconcileTimer) clearInterval(reconcileTimer);
  reconcileTimer = setInterval(async () => {
    await window.refreshStorageStatus();
    window.dispatchEvent(new CustomEvent('shared-data-reconcile'));
  }, SHARED_DATA_RECONCILE_INTERVAL);
}

async function saveAndQuit() {
  if (quitInProgress) return;
  quitInProgress = true;

  try {
    await window.flushPendingAppState();
    await callCommand('quit_app');
  } catch (error) {
    quitInProgress = false;
    throw error;
  }
}

document.querySelectorAll('.tab').forEach((tab) => {
  tab.addEventListener('click', () => setActiveTab(tab.dataset.tab));
});

document.getElementById('btn-minimize').addEventListener('click', async () => {
  await callCommand('window_minimize');
});

document.getElementById('title-bar-drag').addEventListener('mousedown', async (event) => {
  if (event.buttons !== 1) return;
  if (event.target.closest('button, input, textarea, select, a')) return;

  try {
    await currentWindow.startDragging();
  } catch (error) {
    console.error('Failed to start dragging the window:', error);
  }
});

function confirmQuit() {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    const dialog = document.createElement('div');
    dialog.className = 'confirm-dialog';
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    const msg = document.createElement('div');
    msg.className = 'confirm-message';
    msg.textContent = 'Quit TaskTracker Extreme 3000?';
    const actions = document.createElement('div');
    actions.className = 'confirm-actions';
    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'confirm-cancel';
    cancelBtn.type = 'button';
    cancelBtn.textContent = 'Cancel';
    const quitBtn = document.createElement('button');
    quitBtn.className = 'confirm-delete';
    quitBtn.type = 'button';
    quitBtn.textContent = 'Quit';
    actions.appendChild(cancelBtn);
    actions.appendChild(quitBtn);
    dialog.appendChild(msg);
    dialog.appendChild(actions);
    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    function close(result) { overlay.remove(); resolve(result); }
    cancelBtn.addEventListener('click', () => close(false));
    quitBtn.addEventListener('click', () => close(true));
    overlay.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') { e.preventDefault(); close(false); }
    });
    cancelBtn.focus();
  });
}

const closeBtn = document.getElementById('btn-close');
closeBtn.title = 'Save and quit';
closeBtn.setAttribute('aria-label', 'Save and quit');

closeBtn.addEventListener('click', async () => {
  if (!await confirmQuit()) return;
  await saveAndQuit();
});

const pinBtn = document.getElementById('btn-pin');
pinBtn.classList.add('pinned');
pinBtn.addEventListener('click', async () => {
  const isOnTop = await callCommand('toggle_always_on_top');
  pinBtn.classList.toggle('pinned', isOnTop);
});

document.querySelectorAll('.section-header').forEach((header) => {
  function toggleSection() {
    const section = header.closest('.kanban-section');
    if (!section) return;
    section.classList.toggle('collapsed');
    const expanded = !section.classList.contains('collapsed');
    header.setAttribute('aria-expanded', expanded ? 'true' : 'false');
    // If the user manually expands an empty column, reset its auto-collapse timer.
    if (expanded && typeof window.resetEmptyColumnTimer === 'function') {
      const { board, column } = section.dataset;
      if (board && column) window.resetEmptyColumnTimer(board, column);
    }
  }

  header.addEventListener('click', (event) => {
    if (event.target.closest('.clear-done-btn')) return;
    toggleSection();
  });

  header.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      toggleSection();
    }
  });
});

window.addEventListener('blur', () => {
  document.body.classList.add('window-inactive');
});

window.addEventListener('focus', () => {
  document.body.classList.remove('window-inactive');
});

listen('tasks-updated', () => {
  if (typeof loadTasks === 'function') loadTasks();
});

listen('shared-data-changed', (event) => {
  window.dispatchEvent(new CustomEvent('shared-data-changed', {
    detail: event.payload || { files: [] },
  }));
});

listen('navigate-to-tab', (event) => {
  const payload = event.payload || {};
  setActiveTab(payload.tab || 'settings', payload.section);
});

listen('app-close-requested', async () => {
  try {
    if (!await confirmQuit()) return;
    await saveAndQuit();
  } catch (error) {
    console.error('Failed to save and quit:', error);
  }
});

Promise.all([window.refreshStorageStatus(), window.refreshAppMetadata()]).catch((error) => {
  console.error('Failed to initialize app state:', error);
});

callCommand('check_for_update').then((result) => {
  if (result && result.available) {
    window.showAppNotice(
      `v${result.latestVersion} is available — click to download`,
      'info',
      0,
      () => window.openExternal(result.releaseUrl),
    );
  }
}).catch(() => {});

startSharedDataReconcile();

// ── Periodic archive snapshot ─────────────────────────────────────────────────

async function performArchiveSnapshot() {
  try {
    const tasks = typeof window.getCurrentTasksForArchive === 'function'
      ? window.getCurrentTasksForArchive() : [];
    const notes = typeof window.getCurrentNotesForArchive === 'function'
      ? window.getCurrentNotesForArchive() : { meetingNotes: '', generalNotes: '' };
    await callCommand('write_archive_snapshot', {
      tasks,
      meetingNotes: notes.meetingNotes,
      generalNotes: notes.generalNotes,
    });
  } catch (error) {
    console.error('Failed to write archive snapshot:', error);
  }
}

setInterval(performArchiveSnapshot, 3 * 60 * 60 * 1000); // every 3 hours
window.registerSaveHook(performArchiveSnapshot);          // also on quit
