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
let activeDropColumn = null;
const COLUMNS = ['standing', 'priority', 'inprogress', 'todo', 'rainyday', 'done'];

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
  tasks = Array.isArray(document.tasks) ? document.tasks : [];
}

function markTasksDirty() {
  tasksDirty = true;
}

function setDraggingState(active) {
  dragInProgress = active;
  document.body.classList.toggle('kanban-dragging', active);
  if (!active) {
    setActiveDropColumn(null);
  }
}

function setActiveDropColumn(column) {
  activeDropColumn = column;
  document.querySelectorAll('.kanban-section').forEach((section) => {
    section.classList.toggle('drop-target', column !== null && section.dataset.column === column);
  });
}

function isTaskStorageWritable() {
  return taskStorageWritable;
}

function updateTaskControls() {
  const disabled = !isTaskStorageWritable();
  document.getElementById('new-task-input').disabled = disabled;
  document.getElementById('new-task-column').disabled = disabled;
  document.getElementById('btn-add-task').disabled = disabled;
  document.getElementById('btn-sort-alpha').disabled = disabled;
  document.getElementById('btn-clear-done').disabled = disabled;
}

function alphaSortColumn(column) {
  const columnTasks = tasks.filter((task) => task.column === column);
  columnTasks.sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }));
  columnTasks.forEach((task, index) => {
    task.order = index;
  });
}

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
  COLUMNS.forEach((column) => renderColumn(column));
  window.updateTabCount('tasks', tasks.length);
}

function renderColumn(column) {
  const list = document.querySelector(`.task-list[data-column="${column}"]`);
  const countEl = document.querySelector(`.kanban-section[data-column="${column}"] .task-count`);
  const columnTasks = tasks
    .filter((task) => task.column === column)
    .sort((a, b) => a.order - b.order);

  countEl.textContent = `${columnTasks.length}`;
  list.innerHTML = '';
  columnTasks.forEach((task) => list.appendChild(createTaskCard(task)));
}

function createTaskCard(task) {
  const card = document.createElement('div');
  card.className = 'task-card';
  card.dataset.id = task.id;

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

  return card;
}

function addTask(title, column) {
  const columnTasks = tasks.filter((task) => task.column === column);
  const now = new Date().toISOString();
  tasks.push({
    id: `t_${Date.now()}`,
    title,
    notes: '',
    column,
    order: columnTasks.length,
    createdAt: now,
    updatedAt: now,
  });

  alphaSortColumn(column);
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

document.getElementById('btn-clear-done').addEventListener('click', (event) => {
  event.stopPropagation();
  if (!isTaskStorageWritable()) return;

  const doneTasks = tasks.filter((task) => task.column === 'done');
  if (!doneTasks.length) return;

  if (!confirm(`Clear ${doneTasks.length} done ${doneTasks.length === 1 ? 'task' : 'tasks'}?`)) {
    return;
  }

  const updatedAt = new Date().toISOString();
  doneTasks.forEach((task) => recordTaskTombstone(task.id, updatedAt));
  tasks = tasks.filter((task) => task.column !== 'done');
  markTasksDirty();
  saveTasks();
  renderAllColumns();
});

document.getElementById('btn-add-task').addEventListener('click', () => {
  if (!isTaskStorageWritable()) return;

  const input = document.getElementById('new-task-input');
  const column = document.getElementById('new-task-column').value;
  const title = input.value.trim();
  if (!title) return;

  addTask(title, column);
  input.value = '';
  input.focus();
});

document.getElementById('new-task-input').addEventListener('keydown', (event) => {
  if (event.key === 'Enter') document.getElementById('btn-add-task').click();
});

COLUMNS.forEach((column) => {
  const element = document.querySelector(`.task-list[data-column="${column}"]`);
  Sortable.create(element, {
    group: 'kanban',
    animation: 150,
    emptyInsertThreshold: 36,
    ghostClass: 'sortable-ghost',
    chosenClass: 'sortable-chosen',
    dragClass: 'sortable-drag',
    onStart: () => {
      setDraggingState(true);
      setActiveDropColumn(column);
    },
    onMove: (event) => {
      const nextColumn = event.to && event.to.dataset ? event.to.dataset.column : null;
      setActiveDropColumn(nextColumn);
      return true;
    },
    onEnd: async (event) => {
      if (!isTaskStorageWritable()) {
        setDraggingState(false);
        await loadTasks({ silent: true });
        return;
      }

      const taskId = event.item.dataset.id;
      const newColumn = event.to.dataset.column;
      const task = tasks.find((entry) => entry.id === taskId);

      if (task) {
        task.column = newColumn;
        task.updatedAt = new Date().toISOString();
        markTasksDirty();
      }

      [event.from.dataset.column, newColumn].forEach((col) => alphaSortColumn(col));
      saveTasks();
      renderAllColumns();
      setDraggingState(false);
    },
  });
});

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
  if (task) alphaSortColumn(task.column);
  saveTasks();
}

document.addEventListener('click', (event) => {
  if (event.target.closest('.task-card')) return;
  document.querySelectorAll('.task-card.expanded').forEach((card) => {
    const task = tasks.find((entry) => entry.id === card.dataset.id);
    collapseCard(card, task);
  });
});

document.getElementById('btn-sort-alpha').addEventListener('click', () => {
  if (!isTaskStorageWritable()) return;
  COLUMNS.forEach((column) => alphaSortColumn(column));
  markTasksDirty();
  saveTasks();
  renderAllColumns();
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

loadTasks();
