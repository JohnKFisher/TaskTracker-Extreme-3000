const notesArea = document.getElementById('notes-area');
let notesWritable = true;
let notesDocument = {
  schemaVersion: 2,
  revision: 0,
  updatedAt: null,
  updatedBy: null,
  content: '',
};
let notesDirty = false;
let pendingRemoteReload = false;

function applyNotesDocument(document) {
  notesDocument = {
    schemaVersion: document.schemaVersion || 2,
    revision: document.revision || 0,
    updatedAt: document.updatedAt || null,
    updatedBy: document.updatedBy || null,
    content: document.content || '',
  };
}

async function loadNotes(options = {}) {
  const { silent = false } = options;
  if (notesDirty) {
    pendingRemoteReload = true;
    return;
  }

  try {
    const data = await window.callCommand('load_notes');
    applyNotesDocument(data);
    notesArea.value = notesDocument.content;
    notesDirty = false;
    pendingRemoteReload = false;
  } catch (error) {
    notesArea.value = '';
    console.error('Failed to load notes:', error);
    if (!silent) {
      window.showAppNotice(error.message || 'Could not load shared notes.', 'danger', 7000);
    }
  }
}

async function persistNotes() {
  if (!notesWritable || !notesDirty) return;

  try {
    const result = await window.callCommand('save_notes', {
      document: {
        schemaVersion: notesDocument.schemaVersion || 2,
        revision: notesDocument.revision || 0,
        updatedAt: new Date().toISOString(),
        content: notesArea.value,
      },
    });

    if (result.conflict) {
      pendingRemoteReload = true;
      window.showAppNotice(
        'Notes changed on another machine while this draft was still dirty. Your local text is still here, but it was not saved over the remote copy.',
        'warning',
        10000,
      );
      return;
    }

    applyNotesDocument(result.document);
    notesDirty = false;
    if (pendingRemoteReload) {
      pendingRemoteReload = false;
      await loadNotes({ silent: true });
    }
  } catch (error) {
    console.error('Failed to save notes:', error);
    await window.refreshStorageStatus();
    await loadNotes({ silent: true });
  }
}

const saveNotes = window.debounce(persistNotes, 350);

notesArea.addEventListener('input', () => {
  notesDirty = true;
  saveNotes();
});

window.registerSaveHook(persistNotes);

window.addEventListener('storage-status-changed', (event) => {
  const status = event.detail;
  notesWritable = Boolean(status && status.sharedDataAvailable);
  notesArea.disabled = !notesWritable;
  if (!notesWritable) {
    notesArea.placeholder = 'Notes are unavailable until the configured shared-data location is reachable.';
  } else {
    notesArea.placeholder = 'Type your notes here...';
  }
});

window.addEventListener('shared-data-changed', async (event) => {
  const files = event.detail && Array.isArray(event.detail.files) ? event.detail.files : [];
  if (!files.includes('notes.json')) return;

  if (notesDirty) {
    pendingRemoteReload = true;
    window.showAppNotice(
      'Notes were updated on another machine. Finish this draft carefully before reloading.',
      'warning',
      7000,
    );
    return;
  }

  await loadNotes({ silent: true });
});

window.addEventListener('shared-data-reconcile', async () => {
  if (!notesDirty) {
    await loadNotes({ silent: true });
  }
});

loadNotes();
