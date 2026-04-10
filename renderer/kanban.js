// === Kanban Board ===
// Sortable is loaded via <script> tag in index.html (global `Sortable`)

let tasks = [];
const COLUMNS = ['standing', 'priority', 'inprogress', 'todo', 'rainyday', 'done'];

// Sort a single column alphabetically and update order values
function alphaSortColumn(col) {
  const colTasks = tasks.filter(t => t.column === col);
  colTasks.sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }));
  colTasks.forEach((t, i) => t.order = i);
}

// Load tasks from file
async function loadTasks() {
  const data = await invoke('load_file', { filename: 'tasks.json' });
  if (data && data.tasks) {
    tasks = data.tasks;
  } else {
    tasks = [];
  }
  renderAllColumns();
}

// Save tasks to file (debounced)
const saveTasks = debounce(async () => {
  await invoke('save_file', { filename: 'tasks.json', data: { schemaVersion: 1, tasks } });
}, 500);

// Render all columns
function renderAllColumns() {
  COLUMNS.forEach(col => renderColumn(col));
  updateTabCount('tasks', tasks.length);
}

// Render a single column
function renderColumn(column) {
  const list = document.querySelector(`.task-list[data-column="${column}"]`);
  const countEl = document.querySelector(`.kanban-section[data-column="${column}"] .task-count`);

  const columnTasks = tasks
    .filter(t => t.column === column)
    .sort((a, b) => a.order - b.order);

  countEl.textContent = columnTasks.length;

  list.innerHTML = '';
  columnTasks.forEach(task => {
    list.appendChild(createTaskCard(task));
  });
}

// Create a task card element
function createTaskCard(task) {
  const card = document.createElement('div');
  card.className = 'task-card';
  card.dataset.id = task.id;

  const header = document.createElement('div');
  header.className = 'task-card-header';

  const title = document.createElement('span');
  title.className = 'task-title';
  title.textContent = task.title;

  if (task.notes) {
    const noteIndicator = document.createElement('span');
    noteIndicator.className = 'task-has-notes';
    noteIndicator.textContent = '\u{1F4DD}';
    header.appendChild(noteIndicator);
  }

  const deleteBtn = document.createElement('button');
  deleteBtn.className = 'task-delete';
  deleteBtn.innerHTML = '&times;';
  deleteBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    deleteTask(task.id);
  });

  header.appendChild(title);
  header.appendChild(deleteBtn);
  card.appendChild(header);

  // Tooltip on hover (only if has notes and not expanded)
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

  // Double-click to expand/edit
  card.addEventListener('dblclick', (e) => {
    if (e.target.closest('.task-delete') || e.target.closest('.task-notes-area') || e.target.closest('.task-title-input')) return;
    if (card.classList.contains('expanded')) return;

    card.classList.add('expanded');

    const titleSpan = card.querySelector('.task-title');
    if (titleSpan) {
      const input = document.createElement('input');
      input.type = 'text';
      input.className = 'task-title-input';
      input.value = task.title;
      input.addEventListener('click', e => e.stopPropagation());
      input.addEventListener('dblclick', e => e.stopPropagation());
      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          task.title = input.value.trim() || task.title;
          task.updatedAt = new Date().toISOString();
          saveTasks();
          collapseCard(card, task);
        }
      });
      titleSpan.replaceWith(input);
      input.focus();
      input.select();
    }

    const notesArea = document.createElement('textarea');
    notesArea.className = 'task-notes-area';
    notesArea.value = task.notes || '';
    notesArea.placeholder = 'Add notes...';
    notesArea.addEventListener('click', e => e.stopPropagation());
    notesArea.addEventListener('dblclick', e => e.stopPropagation());
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

// Add a new task
function addTask(title, column) {
  const columnTasks = tasks.filter(t => t.column === column);
  const task = {
    id: 't_' + Date.now(),
    title,
    notes: '',
    column,
    order: columnTasks.length,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  tasks.push(task);
  alphaSortColumn(column);
  saveTasks();
  renderColumn(column);
}

// Delete a task (with confirmation for non-done tasks)
function deleteTask(id) {
  const task = tasks.find(t => t.id === id);
  if (task && task.column !== 'done') {
    if (!confirm(`Delete "${task.title}"?`)) return;
  }
  tasks = tasks.filter(t => t.id !== id);
  saveTasks();
  renderAllColumns();
}

// Clear done tasks
document.getElementById('btn-clear-done').addEventListener('click', (e) => {
  e.stopPropagation();
  tasks = tasks.filter(t => t.column !== 'done');
  saveTasks();
  renderColumn('done');
});

// Add task button
document.getElementById('btn-add-task').addEventListener('click', () => {
  const input = document.getElementById('new-task-input');
  const column = document.getElementById('new-task-column').value;
  const title = input.value.trim();
  if (title) {
    addTask(title, column);
    input.value = '';
    input.focus();
  }
});

// Enter key to add task
document.getElementById('new-task-input').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    document.getElementById('btn-add-task').click();
  }
});

// Initialize SortableJS on each column
COLUMNS.forEach(column => {
  const el = document.querySelector(`.task-list[data-column="${column}"]`);
  Sortable.create(el, {
    group: 'kanban',
    animation: 150,
    ghostClass: 'sortable-ghost',
    chosenClass: 'sortable-chosen',
    dragClass: 'sortable-drag',
    onEnd: (evt) => {
      const taskId = evt.item.dataset.id;
      const newColumn = evt.to.dataset.column;

      const task = tasks.find(t => t.id === taskId);
      if (task) {
        task.column = newColumn;
        task.updatedAt = new Date().toISOString();
      }

      [evt.from.dataset.column, newColumn].forEach(col => {
        alphaSortColumn(col);
        const list = document.querySelector(`.task-list[data-column="${col}"]`);
        const cards = list.querySelectorAll('.task-card');
        const count = document.querySelector(`.kanban-section[data-column="${col}"] .task-count`);
        count.textContent = cards.length;
      });

      saveTasks();
      [evt.from.dataset.column, newColumn].forEach(col => renderColumn(col));
    },
  });
});

// Collapse a card back to its compact state
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

// Click outside any expanded task to collapse it
document.addEventListener('click', (e) => {
  if (e.target.closest('.task-card')) return;
  document.querySelectorAll('.task-card.expanded').forEach(card => {
    const task = tasks.find(t => t.id === card.dataset.id);
    collapseCard(card, task);
  });
});

// Sort all columns alphabetically
document.getElementById('btn-sort-alpha').addEventListener('click', () => {
  COLUMNS.forEach(col => alphaSortColumn(col));
  saveTasks();
  renderAllColumns();
});

// Load on startup
loadTasks();
