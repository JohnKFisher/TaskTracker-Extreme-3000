// === App Initialization & Tab Switching ===

// Tauri API globals — available because withGlobalTauri: true in tauri.conf.json
const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

// Debounce utility
function debounce(fn, ms) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

// Update tab entry count
function updateTabCount(tabName, count) {
  const el = document.getElementById('tab-count-' + tabName);
  if (el) el.textContent = count > 0 ? `(${count})` : '';
}

// Tab switching
document.querySelectorAll('.tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));

    tab.classList.add('active');
    const tabId = 'tab-' + tab.dataset.tab;
    document.getElementById(tabId).classList.add('active');
  });
});

// Title bar buttons
document.getElementById('btn-minimize').addEventListener('click', () => {
  invoke('window_minimize');
});

document.getElementById('btn-close').addEventListener('click', () => {
  invoke('hide_window');
});

const pinBtn = document.getElementById('btn-pin');
pinBtn.classList.add('pinned'); // starts as always-on-top
pinBtn.addEventListener('click', async () => {
  const isOnTop = await invoke('toggle_always_on_top');
  pinBtn.classList.toggle('pinned', isOnTop);
});

// Section collapse toggle
document.querySelectorAll('.section-header').forEach(header => {
  header.addEventListener('click', (e) => {
    if (e.target.closest('.clear-done-btn')) return;
    const section = header.closest('.kanban-section');
    section.classList.toggle('collapsed');
  });
});

// Window opacity on focus/blur (frameless window, so body opacity dims the content)
window.addEventListener('blur', () => {
  document.body.style.opacity = '0.7';
});
window.addEventListener('focus', () => {
  document.body.style.opacity = '1';
});

// Listen for task updates from backend (e.g., quick-add)
listen('tasks-updated', () => {
  if (typeof loadTasks === 'function') {
    loadTasks();
  }
});
