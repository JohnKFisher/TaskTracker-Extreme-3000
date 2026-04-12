let tasks = [];
let taskStorageWritable = true;
const COLUMNS = ['standing', 'priority', 'inprogress', 'todo', 'rainyday', 'done'];

function taskDocument() {
  return {
    schemaVersion: 1,
    tasks,
  };
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

async function loadTasks() {
  try {
    const data = await window.callCommand('load_tasks');
    tasks = Array.isArray(data.tasks) ? data.tasks : [];
  } catch (error) {
    tasks = [];
    console.error('Failed to load tasks:', error);
  }

  renderAllColumns();
  updateTaskControls();
}

async function persistTasks() {
  if (!isTaskStorageWritable()) return;

  try {
    await window.callCommand('save_tasks', { document: taskDocument() });
  } catch (error) {
    console.error('Failed to save tasks:', error);
    await window.refreshStorageStatus();
    await loadTasks();
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
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        task.title = input.value.trim() || task.title;
        task.updatedAt = new Date().toISOString();
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
  tasks.push({
    id: `t_${Date.now()}`,
    title,
    notes: '',
    column,
    order: columnTasks.length,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  });

  alphaSortColumn(column);
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

  tasks = tasks.filter((entry) => entry.id !== id);
  saveTasks();
  renderAllColumns();
}

document.getElementById('btn-clear-done').addEventListener('click', (event) => {
  event.stopPropagation();
  if (!isTaskStorageWritable()) return;

  const doneCount = tasks.filter((task) => task.column === 'done').length;
  if (!doneCount) return;

  if (!confirm(`Clear ${doneCount} done ${doneCount === 1 ? 'task' : 'tasks'}?`)) {
    return;
  }

  tasks = tasks.filter((task) => task.column !== 'done');
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
    ghostClass: 'sortable-ghost',
    chosenClass: 'sortable-chosen',
    dragClass: 'sortable-drag',
    onEnd: async (event) => {
      if (!isTaskStorageWritable()) {
        await loadTasks();
        return;
      }

      const taskId = event.item.dataset.id;
      const newColumn = event.to.dataset.column;
      const task = tasks.find((entry) => entry.id === taskId);

      if (task) {
        task.column = newColumn;
        task.updatedAt = new Date().toISOString();
      }

      [event.from.dataset.column, newColumn].forEach((col) => alphaSortColumn(col));
      saveTasks();
      renderAllColumns();
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
  saveTasks();
  renderAllColumns();
});

window.addEventListener('storage-status-changed', (event) => {
  const status = event.detail;
  taskStorageWritable = Boolean(status && status.sharedDataAvailable);
  updateTaskControls();
  renderAllColumns();
});

window.registerBeforeQuitHook(() => {
  document.querySelectorAll('.task-card.expanded').forEach((card) => {
    const task = tasks.find((entry) => entry.id === card.dataset.id);
    collapseCard(card, task);
  });
});
window.registerSaveHook(persistTasks);

loadTasks();
