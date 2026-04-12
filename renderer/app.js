const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

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

function updateTabCount(tabName, count) {
  const el = document.getElementById(`tab-count-${tabName}`);
  if (el) el.textContent = count > 0 ? `(${count})` : '';
}

window.updateTabCount = updateTabCount;

function setActiveTab(tabName, section) {
  document.querySelectorAll('.tab').forEach((tab) => {
    const active = tab.dataset.tab === tabName;
    tab.classList.toggle('active', active);
    tab.setAttribute('aria-selected', active ? 'true' : 'false');
  });

  document.querySelectorAll('.tab-content').forEach((panel) => {
    panel.classList.toggle('active', panel.id === `tab-${tabName}`);
  });

  if (section) {
    const target = document.getElementById(`${section}-section`);
    if (target) target.scrollIntoView({ block: 'start', behavior: 'smooth' });
  }
}

window.setActiveTab = setActiveTab;

function renderStorageBanner(status) {
  const banner = document.getElementById('storage-banner');
  window.currentStorageStatus = status;

  if (status.mode === 'syncUnavailable' || status.mode === 'localUnavailable') {
    banner.textContent = status.message || 'Shared data is currently unavailable.';
    banner.classList.remove('hidden');
  } else {
    banner.textContent = '';
    banner.classList.add('hidden');
  }

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

document.querySelectorAll('.tab').forEach((tab) => {
  tab.addEventListener('click', () => setActiveTab(tab.dataset.tab));
});

document.getElementById('btn-minimize').addEventListener('click', async () => {
  await callCommand('window_minimize');
});

document.getElementById('btn-close').addEventListener('click', async () => {
  await callCommand('hide_window');
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

listen('navigate-to-tab', (event) => {
  const payload = event.payload || {};
  setActiveTab(payload.tab || 'settings', payload.section);
});

Promise.all([window.refreshStorageStatus(), window.refreshAppMetadata()]).catch((error) => {
  console.error('Failed to initialize app state:', error);
});
