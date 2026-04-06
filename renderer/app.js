// === App Initialization & Tab Switching ===

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
  window.api.minimize();
});

document.getElementById('btn-close').addEventListener('click', () => {
  window.api.close();
});

const pinBtn = document.getElementById('btn-pin');
pinBtn.classList.add('pinned'); // starts as always-on-top
pinBtn.addEventListener('click', async () => {
  const isOnTop = await window.api.toggleAlwaysOnTop();
  pinBtn.classList.toggle('pinned', isOnTop);
});

// Section collapse toggle
document.querySelectorAll('.section-header').forEach(header => {
  header.addEventListener('click', (e) => {
    // Don't collapse if clicking clear button
    if (e.target.closest('.clear-done-btn')) return;
    const section = header.closest('.kanban-section');
    section.classList.toggle('collapsed');
  });
});

// Listen for task updates from main process (e.g., quick-add)
window.api.onTasksUpdated(() => {
  if (typeof loadTasks === 'function') {
    loadTasks();
  }
});
