const emptyColumnSince = new Map(); // "board:column" → timestamp when it became empty
const AUTO_COLLAPSE_MS = 5 * 60 * 1000; // 5 minutes
const DONE_AUTO_DELETE_MS = 8 * 24 * 60 * 60 * 1000; // 8 days

let tasks = [];
let taskDocumentState = {
  schemaVersion: 2,
  revision: 0,
  updatedAt: null,
  updatedBy: null,
  tombstones: [],
};
let taskStorageWritable = true;
let tasksDirty = false;
let pendingRemoteTaskReload = false;
let taskSaveInFlight = false;
let dragInProgress = false;
let activeDropTarget = null;
const COLUMNS = ['standing', 'priority', 'inprogress', 'todo', 'rainyday', 'done'];
const BOARDS = ['personal', 'work'];
const COLUMN_LABELS = {
  standing: 'Standing',
  priority: 'Priority',
  inprogress: 'In Progress',
  todo: 'To-Do',
  rainyday: 'Rainy Day',
  done: 'Done',
};

function normalizeBoard(board) {
  return board === 'personal' ? 'personal' : 'work';
}

function taskDocument() {
  return {
    schemaVersion: taskDocumentState.schemaVersion || 2,
    revision: taskDocumentState.revision || 0,
    updatedAt: new Date().toISOString(),
    tasks,
    tombstones: taskDocumentState.tombstones || [],
  };
}

function applyTaskDocument(document) {
  taskDocumentState = {
    schemaVersion: document.schemaVersion || 2,
    revision: document.revision || 0,
    updatedAt: document.updatedAt || null,
    updatedBy: document.updatedBy || null,
    tombstones: Array.isArray(document.tombstones) ? document.tombstones : [],
  };
  tasks = Array.isArray(document.tasks)
    ? document.tasks.map((task) => ({ ...task, board: normalizeBoard(task.board) }))
    : [];
}

function markTasksDirty() {
  tasksDirty = true;
}

function dropTargetKey(board, column) {
  return board && column ? `${board}:${column}` : null;
}

function setDraggingState(active) {
  dragInProgress = active;
  document.body.classList.toggle('kanban-dragging', active);
  if (!active) {
    setActiveDropTarget(null, null);
  }
}

function setActiveDropTarget(board, column) {
  activeDropTarget = dropTargetKey(board, column);
  document.querySelectorAll('.kanban-section').forEach((section) => {
    const matches = activeDropTarget !== null
      && section.dataset.board === board
      && section.dataset.column === column;
    section.classList.toggle('drop-target', matches);
  });
}

function isTaskStorageWritable() {
  return taskStorageWritable;
}

function eachBoardControl(callback) {
  document.querySelectorAll('.add-task-bar').forEach((bar) => {
    const board = normalizeBoard(bar.dataset.board);
    callback(board, bar);
  });
}

function updateTaskControls() {
  const disabled = !isTaskStorageWritable();
  eachBoardControl((board, bar) => {
    const input = bar.querySelector(`.new-task-input[data-board="${board}"]`);
    const select = bar.querySelector(`.new-task-column[data-board="${board}"]`);
    const addButton = bar.querySelector(`.btn-add-task[data-board="${board}"]`);
    const sortButton = bar.querySelector(`.btn-sort-alpha[data-board="${board}"]`);
    const clearDoneButton = document.querySelector(`.clear-done-btn[data-board="${board}"]`);
    if (input) input.disabled = disabled;
    if (select) select.disabled = disabled;
    if (addButton) addButton.disabled = disabled;
    if (sortButton) sortButton.disabled = disabled;
    if (clearDoneButton) clearDoneButton.disabled = disabled;
  });
}

function alphaSortColumn(board, column) {
  const normalizedBoard = normalizeBoard(board);
  const columnTasks = tasks.filter((task) => task.board === normalizedBoard && task.column === column);
  columnTasks.sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }));
  columnTasks.forEach((task, index) => {
    task.order = index;
  });
}

// ── Context menu ──────────────────────────────────────────────────────────────

const taskContextMenu = document.createElement('div');
taskContextMenu.id = 'task-context-menu';
taskContextMenu.setAttribute('role', 'menu');
taskContextMenu.setAttribute('aria-label', 'Move task to…');
document.body.appendChild(taskContextMenu);

function dismissContextMenu() {
  taskContextMenu.style.display = 'none';
  taskContextMenu.innerHTML = '';
}

function showContextMenu(event, task) {
  dismissContextMenu();

  const label = document.createElement('span');
  label.className = 'context-menu-label';
  label.textContent = 'Move to…';
  taskContextMenu.appendChild(label);

  COLUMNS.filter((col) => col !== task.column).forEach((col) => {
    const btn = document.createElement('button');
    btn.className = 'context-menu-item';
    btn.type = 'button';
    btn.textContent = COLUMN_LABELS[col] || col;
    btn.setAttribute('role', 'menuitem');
    btn.addEventListener('click', () => {
      const sourceColumn = task.column;
      applyTaskColumnChange(task, col);
      alphaSortColumn(task.board, sourceColumn);
      alphaSortColumn(task.board, col);
      markTasksDirty();
      saveTasks();
      renderAllColumns();
      dismissContextMenu();
    });
    taskContextMenu.appendChild(btn);
  });

  // Position at cursor, clamped to viewport
  const menuWidth = 150;
  const menuHeight = taskContextMenu.children.length * 28 + 24;
  const x = Math.min(event.clientX, window.innerWidth - menuWidth - 8);
  const y = Math.min(event.clientY, window.innerHeight - menuHeight - 8);
  taskContextMenu.style.left = `${x}px`;
  taskContextMenu.style.top = `${y}px`;
  taskContextMenu.style.display = 'block';
}

document.addEventListener('click', dismissContextMenu);
document.addEventListener('keydown', (e) => { if (e.key === 'Escape') dismissContextMenu(); });
window.addEventListener('blur', dismissContextMenu);

// ──────────────────────────────────────────────────────────────────────────────

function recordTaskTombstone(taskId, updatedAt) {
  const tombstones = Array.isArray(taskDocumentState.tombstones) ? [...taskDocumentState.tombstones] : [];
  const filtered = tombstones.filter((entry) => entry.id !== taskId);
  filtered.push({
    id: taskId,
    updatedAt,
  });
  taskDocumentState.tombstones = filtered;
}

async function loadTasks(options = {}) {
  const { silent = false } = options;

  if (tasksDirty) {
    pendingRemoteTaskReload = true;
    return;
  }

  try {
    const data = await window.callCommand('load_tasks');
    applyTaskDocument(data);
    tasksDirty = false;
    pendingRemoteTaskReload = false;
  } catch (error) {
    tasks = [];
    console.error('Failed to load tasks:', error);
    if (!silent) {
      window.showAppNotice(error.message || 'Could not load shared tasks.', 'danger', 7000);
    }
  }

  renderAllColumns();
  updateTaskControls();
  purgeStaleDoneTasks();
}

async function persistTasks() {
  if (!isTaskStorageWritable() || !tasksDirty) return;

  taskSaveInFlight = true;
  try {
    const result = await window.callCommand('save_tasks', { document: taskDocument() });
    applyTaskDocument(result.document);
    tasksDirty = false;
    if (Array.isArray(result.conflictIds) && result.conflictIds.length > 0) {
      window.showAppNotice(
        `${result.conflictIds.length} task change${result.conflictIds.length === 1 ? '' : 's'} merged from another machine.`,
        'warning',
        8000,
      );
    }
    renderAllColumns();
    if (pendingRemoteTaskReload) {
      pendingRemoteTaskReload = false;
      await loadTasks({ silent: true });
    }
  } catch (error) {
    console.error('Failed to save tasks:', error);
    await window.refreshStorageStatus();
    await loadTasks({ silent: true });
  } finally {
    taskSaveInFlight = false;
  }
}

const saveTasks = window.debounce(persistTasks, 350);

function renderAllColumns() {
  BOARDS.forEach((board) => {
    COLUMNS.forEach((column) => renderColumn(board, column));
    window.updateTabCount(board === 'work' ? 'tasks' : 'personal', tasks.filter((task) => task.board === board).length);
  });
}

function renderColumn(board, column) {
  const list = document.querySelector(`.task-list[data-board="${board}"][data-column="${column}"]`);
  const countEl = document.querySelector(`.kanban-section[data-board="${board}"][data-column="${column}"] .task-count`);
  if (!list || !countEl) return;

  const columnTasks = tasks
    .filter((task) => task.board === board && task.column === column)
    .sort((a, b) => a.order - b.order);

  countEl.textContent = `${columnTasks.length}`;
  list.innerHTML = '';
  columnTasks.forEach((task) => list.appendChild(createTaskCard(task)));
  updateEmptyColumnTracking(board, column, columnTasks.length);
}

function createTaskCard(task) {
  const card = document.createElement('div');
  card.className = 'task-card';
  card.dataset.id = task.id;
  card.dataset.board = task.board;

  const header = document.createElement('div');
  header.className = 'task-card-header';

  if (task.notes) {
    const noteIndicator = document.createElement('span');
    noteIndicator.className = 'task-has-notes';
    noteIndicator.textContent = '📝';
    noteIndicator.setAttribute('aria-hidden', 'true');
    header.appendChild(noteIndicator);
  }

  const title = document.createElement('span');
  title.className = 'task-title';
  title.textContent = task.title;
  header.appendChild(title);

  const deleteBtn = document.createElement('button');
  deleteBtn.className = 'task-delete';
  deleteBtn.type = 'button';
  deleteBtn.innerHTML = '&times;';
  deleteBtn.disabled = !isTaskStorageWritable();
  deleteBtn.setAttribute('aria-label', `Delete task ${task.title}`);
  deleteBtn.addEventListener('click', (event) => {
    event.stopPropagation();
    deleteTask(task.id);
  });
  header.appendChild(deleteBtn);

  card.appendChild(header);

  let tooltip = null;
  card.addEventListener('mouseenter', () => {
    if (task.notes && !card.querySelector('.task-notes-area')) {
      tooltip = document.createElement('div');
      tooltip.className = 'task-tooltip';
      tooltip.textContent = task.notes;
      card.appendChild(tooltip);
    }
  });

  card.addEventListener('mouseleave', () => {
    if (tooltip) {
      tooltip.remove();
      tooltip = null;
    }
  });

  card.addEventListener('dblclick', (event) => {
    if (!isTaskStorageWritable()) return;
    if (event.target.closest('.task-delete') || event.target.closest('.task-notes-area') || event.target.closest('.task-title-input')) return;
    if (card.classList.contains('expanded')) return;

    card.classList.add('expanded');

    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'task-title-input';
    input.value = task.title;
    input.addEventListener('click', (e) => e.stopPropagation());
    input.addEventListener('dblclick', (e) => e.stopPropagation());
    input.addEventListener('input', () => {
      task.title = input.value;
      task.updatedAt = new Date().toISOString();
      markTasksDirty();
    });
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        task.title = input.value.trim() || task.title;
        task.updatedAt = new Date().toISOString();
        markTasksDirty();
        saveTasks();
        collapseCard(card, task);
      }
    });
    title.replaceWith(input);
    input.focus();
    input.select();

    const notesArea = document.createElement('textarea');
    notesArea.className = 'task-notes-area';
    notesArea.value = task.notes || '';
    notesArea.placeholder = 'Add notes...';
    notesArea.addEventListener('click', (e) => e.stopPropagation());
    notesArea.addEventListener('dblclick', (e) => e.stopPropagation());
    notesArea.addEventListener('input', () => {
      task.notes = notesArea.value;
      task.updatedAt = new Date().toISOString();
      markTasksDirty();
      saveTasks();
    });
    card.appendChild(notesArea);

    if (tooltip) {
      tooltip.remove();
      tooltip = null;
    }
  });

  card.addEventListener('contextmenu', (event) => {
    if (!isTaskStorageWritable()) return;
    event.preventDefault();
    event.stopPropagation();
    showContextMenu(event, task);
  });

  return card;
}

function addTask(board, title, column) {
  const normalizedBoard = normalizeBoard(board);
  const columnTasks = tasks.filter((task) => task.board === normalizedBoard && task.column === column);
  const now = new Date().toISOString();
  tasks.push({
    id: `t_${Date.now()}`,
    title,
    notes: '',
    column,
    order: columnTasks.length,
    board: normalizedBoard,
    createdAt: now,
    updatedAt: now,
  });

  alphaSortColumn(normalizedBoard, column);
  markTasksDirty();
  saveTasks();
  renderAllColumns();
}

function deleteTask(id) {
  if (!isTaskStorageWritable()) return;

  const task = tasks.find((entry) => entry.id === id);
  if (!task) return;

  if (task.column !== 'done' && !confirm(`Delete "${task.title}"?`)) {
    return;
  }

  const updatedAt = new Date().toISOString();
  tasks = tasks.filter((entry) => entry.id !== id);
  recordTaskTombstone(id, updatedAt);
  markTasksDirty();
  saveTasks();
  renderAllColumns();
}

function clearDoneTasks(board) {
  if (!isTaskStorageWritable()) return;

  const normalizedBoard = normalizeBoard(board);
  const doneTasks = tasks.filter((task) => task.board === normalizedBoard && task.column === 'done');
  if (!doneTasks.length) return;

  if (!confirm(`Clear ${doneTasks.length} done ${doneTasks.length === 1 ? 'task' : 'tasks'}?`)) {
    return;
  }

  const updatedAt = new Date().toISOString();
  doneTasks.forEach((task) => recordTaskTombstone(task.id, updatedAt));
  tasks = tasks.filter((task) => !(task.board === normalizedBoard && task.column === 'done'));
  markTasksDirty();
  saveTasks();
  renderAllColumns();
}

function handleAddTask(board) {
  if (!isTaskStorageWritable()) return;

  const input = document.querySelector(`.new-task-input[data-board="${board}"]`);
  const columnSelect = document.querySelector(`.new-task-column[data-board="${board}"]`);
  if (!input || !columnSelect) return;

  const title = input.value.trim();
  if (!title) return;

  addTask(board, title, columnSelect.value);
  input.value = '';
  input.focus();
}

// Apply a column move to a task, tracking done-entry timestamp.
function applyTaskColumnChange(task, newColumn) {
  const now = new Date().toISOString();
  if (newColumn === 'done' && task.column !== 'done') {
    task.movedToDoneAt = now;
  } else if (newColumn !== 'done') {
    task.movedToDoneAt = null;
  }
  task.column = newColumn;
  task.updatedAt = now;
}

// Delete done tasks whose movedToDoneAt is older than 8 days.
function purgeStaleDoneTasks() {
  if (!isTaskStorageWritable()) return;
  const cutoff = Date.now() - DONE_AUTO_DELETE_MS;
  const stale = tasks.filter(
    (t) => t.column === 'done'
      && t.movedToDoneAt
      && new Date(t.movedToDoneAt).getTime() <= cutoff,
  );
  if (!stale.length) return;
  const updatedAt = new Date().toISOString();
  stale.forEach((t) => recordTaskTombstone(t.id, updatedAt));
  tasks = tasks.filter((t) => !stale.some((s) => s.id === t.id));
  markTasksDirty();
  saveTasks();
  renderAllColumns();
}

setInterval(purgeStaleDoneTasks, 60 * 60 * 1000); // check once per hour

// Expose current tasks for the archive feature.
window.getCurrentTasksForArchive = function getCurrentTasksForArchive() {
  return tasks.slice();
};

function initializeBoardControls() {
  document.querySelectorAll('.clear-done-btn').forEach((button) => {
    button.addEventListener('click', (event) => {
      event.stopPropagation();
      clearDoneTasks(button.dataset.board);
    });
  });

  document.querySelectorAll('.btn-add-task').forEach((button) => {
    button.addEventListener('click', () => {
      handleAddTask(button.dataset.board);
    });
  });

  document.querySelectorAll('.new-task-input').forEach((input) => {
    input.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') handleAddTask(input.dataset.board);
    });
  });

  document.querySelectorAll('.btn-sort-alpha').forEach((button) => {
    button.addEventListener('click', () => {
      if (!isTaskStorageWritable()) return;
      const board = normalizeBoard(button.dataset.board);
      COLUMNS.forEach((column) => alphaSortColumn(board, column));
      markTasksDirty();
      saveTasks();
      renderAllColumns();
    });
  });
}

// ── Auto-collapse empty columns ───────────────────────────────────────────────

function updateEmptyColumnTracking(board, column, taskCount) {
  const key = `${board}:${column}`;
  if (taskCount === 0) {
    if (!emptyColumnSince.has(key)) {
      emptyColumnSince.set(key, Date.now());
    }
  } else {
    emptyColumnSince.delete(key);
  }
}

function checkAutoCollapse() {
  const now = Date.now();
  emptyColumnSince.forEach((since, key) => {
    if (now - since < AUTO_COLLAPSE_MS) return;
    const [board, column] = key.split(':');
    const section = document.querySelector(
      `.kanban-section[data-board="${board}"][data-column="${column}"]`,
    );
    if (section && !section.classList.contains('collapsed')) {
      section.classList.add('collapsed');
      const header = section.querySelector('.section-header');
      if (header) header.setAttribute('aria-expanded', 'false');
    }
  });
}

setInterval(checkAutoCollapse, 30 * 1000);

// Called from app.js when user manually expands a section, giving it a fresh window.
window.resetEmptyColumnTimer = function resetEmptyColumnTimer(board, column) {
  const key = `${board}:${column}`;
  if (emptyColumnSince.has(key)) {
    emptyColumnSince.set(key, Date.now());
  }
};

// ──────────────────────────────────────────────────────────────────────────────

function initializeSortableBoards() {
  document.querySelectorAll('.task-list').forEach((element) => {
    const board = normalizeBoard(element.dataset.board);
    Sortable.create(element, {
      group: `kanban-${board}`,
      animation: 150,
      forceFallback: true,
      fallbackOnBody: false,
      fallbackTolerance: 2,
      fallbackClass: 'sortable-fallback',
      emptyInsertThreshold: 36,
      ghostClass: 'sortable-ghost',
      chosenClass: 'sortable-chosen',
      dragClass: 'sortable-drag',
      onStart: (event) => {
        const startBoard = normalizeBoard(event.from?.dataset?.board);
        const startColumn = event.from?.dataset?.column || null;
        setDraggingState(true);
        setActiveDropTarget(startBoard, startColumn);
      },
      onMove: (event) => {
        const nextBoard = normalizeBoard(event.to?.dataset?.board);
        const nextColumn = event.to?.dataset?.column || null;
        setActiveDropTarget(nextBoard, nextColumn);
        return true;
      },
      onEnd: async (event) => {
        try {
          if (!isTaskStorageWritable()) {
            await loadTasks({ silent: true });
            return;
          }

          const taskId = event.item.dataset.id;
          const sourceBoard = normalizeBoard(event.from?.dataset?.board);
          const sourceColumn = event.from?.dataset?.column || null;
          const newBoard = normalizeBoard(event.to?.dataset?.board);
          const newColumn = event.to?.dataset?.column || null;
          const task = tasks.find((entry) => entry.id === taskId);

          if (!task || !newColumn) {
            await loadTasks({ silent: true });
            return;
          }

          task.board = newBoard;
          applyTaskColumnChange(task, newColumn);
          markTasksDirty();

          [
            dropTargetKey(sourceBoard, sourceColumn),
            dropTargetKey(newBoard, newColumn),
          ]
            .filter((value, index, array) => value && array.indexOf(value) === index)
            .forEach((value) => {
              const [affectedBoard, affectedColumn] = value.split(':');
              alphaSortColumn(affectedBoard, affectedColumn);
            });

          saveTasks();
          renderAllColumns();
        } finally {
          setDraggingState(false);
        }
      },
    });
  });
}

function collapseCard(card, task) {
  if (!card.classList.contains('expanded')) return;

  const notesArea = card.querySelector('.task-notes-area');
  const titleInput = card.querySelector('.task-title-input');

  if (titleInput) {
    if (task) {
      task.title = titleInput.value.trim() || task.title;
      task.updatedAt = new Date().toISOString();
      markTasksDirty();
    }
    const newTitle = document.createElement('span');
    newTitle.className = 'task-title';
    newTitle.textContent = task ? task.title : titleInput.value;
    titleInput.replaceWith(newTitle);
  }

  if (notesArea) notesArea.remove();
  card.classList.remove('expanded');
  if (task) alphaSortColumn(task.board, task.column);
  saveTasks();
}

document.addEventListener('click', (event) => {
  if (event.target.closest('.task-card')) return;
  document.querySelectorAll('.task-card.expanded').forEach((card) => {
    const task = tasks.find((entry) => entry.id === card.dataset.id);
    collapseCard(card, task);
  });
});

window.addEventListener('storage-status-changed', (event) => {
  const status = event.detail;
  taskStorageWritable = Boolean(status && status.sharedDataAvailable);
  updateTaskControls();
  renderAllColumns();
});

window.addEventListener('shared-data-changed', async (event) => {
  const files = event.detail && Array.isArray(event.detail.files) ? event.detail.files : [];
  if (!files.includes('tasks.json')) return;

  if (tasksDirty || dragInProgress || taskSaveInFlight) {
    pendingRemoteTaskReload = true;
    return;
  }

  await loadTasks({ silent: true });
});

window.addEventListener('shared-data-reconcile', async () => {
  if (!tasksDirty && !dragInProgress && !taskSaveInFlight) {
    await loadTasks({ silent: true });
  }
});

window.registerBeforeQuitHook(() => {
  document.querySelectorAll('.task-card.expanded').forEach((card) => {
    const task = tasks.find((entry) => entry.id === card.dataset.id);
    collapseCard(card, task);
  });
});

window.registerSaveHook(persistTasks);

initializeBoardControls();
initializeSortableBoards();
loadTasks();
