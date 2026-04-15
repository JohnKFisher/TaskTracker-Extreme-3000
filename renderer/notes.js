const meetingArea = document.getElementById('notes-meeting');
const generalArea = document.getElementById('notes-general');
let notesWritable = true;
let notesDocument = {
  schemaVersion: 3,
  revision: 0,
  updatedAt: null,
  updatedBy: null,
  meetingNotes: '',
  generalNotes: '',
};
let notesDirty = false;
let pendingRemoteReload = false;

function applyNotesDocument(document) {
  notesDocument = {
    schemaVersion: document.schemaVersion || 3,
    revision: document.revision || 0,
    updatedAt: document.updatedAt || null,
    updatedBy: document.updatedBy || null,
    // Rust normalizes legacy `content` into `generalNotes` before returning.
    meetingNotes: document.meetingNotes || '',
    generalNotes: document.generalNotes || '',
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
    meetingArea.value = notesDocument.meetingNotes;
    generalArea.value = notesDocument.generalNotes;
    notesDirty = false;
    pendingRemoteReload = false;
  } catch (error) {
    meetingArea.value = '';
    generalArea.value = '';
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
        schemaVersion: notesDocument.schemaVersion || 3,
        revision: notesDocument.revision || 0,
        updatedAt: new Date().toISOString(),
        content: '',
        meetingNotes: meetingArea.value,
        generalNotes: generalArea.value,
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

function markNotesDirty() {
  notesDirty = true;
  saveNotes();
}

meetingArea.addEventListener('input', markNotesDirty);
generalArea.addEventListener('input', markNotesDirty);

window.registerSaveHook(persistNotes);

// Expose current notes state for the archive feature.
window.getCurrentNotesForArchive = function getCurrentNotesForArchive() {
  return {
    meetingNotes: meetingArea.value,
    generalNotes: generalArea.value,
  };
};

function setNotesEnabled(enabled) {
  meetingArea.disabled = !enabled;
  generalArea.disabled = !enabled;
  const offlinePlaceholder = 'Notes are unavailable until the configured shared-data location is reachable.';
  if (enabled) {
    meetingArea.placeholder = 'Notes for your next meeting...';
    generalArea.placeholder = 'General notes...';
  } else {
    meetingArea.placeholder = offlinePlaceholder;
    generalArea.placeholder = offlinePlaceholder;
  }
}

window.addEventListener('storage-status-changed', (event) => {
  const status = event.detail;
  notesWritable = Boolean(status && status.sharedDataAvailable);
  setNotesEnabled(notesWritable);
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
