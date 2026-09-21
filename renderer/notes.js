const meetingArea = document.getElementById('notes-meeting');
// The Long-Term section is still stored under `generalNotes` — schema v4 relabelled it
// from "General" without renaming the field, so older builds sharing the same synced
// notes.json keep reading and writing that text correctly. See NotesDocument in main.rs.
const shortTermArea = document.getElementById('notes-short-term');
const generalArea = document.getElementById('notes-general');
let notesWritable = true;
let notesDocument = {
  schemaVersion: 4,
  revision: 0,
  updatedAt: null,
  updatedBy: null,
  meetingNotes: '',
  shortTermNotes: '',
  generalNotes: '',
};
let notesDirty = false;
let pendingRemoteReload = false;

function applyNotesDocument(document) {
  notesDocument = {
    schemaVersion: document.schemaVersion || 4,
    revision: document.revision || 0,
    updatedAt: document.updatedAt || null,
    updatedBy: document.updatedBy || null,
    // Rust normalizes legacy `content` into `generalNotes` before returning, and a v3
    // document simply arrives with no `shortTermNotes`.
    meetingNotes: document.meetingNotes || '',
    shortTermNotes: document.shortTermNotes || '',
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
    shortTermArea.value = notesDocument.shortTermNotes;
    generalArea.value = notesDocument.generalNotes;
    notesDirty = false;
    pendingRemoteReload = false;
  } catch (error) {
    meetingArea.value = '';
    shortTermArea.value = '';
    generalArea.value = '';
    console.error('Failed to load notes:', error);
    if (!silent) {
      window.showAppNotice(error.message || 'Could not load shared notes.', 'danger', 7000);
    }
  }
}

let notesSaveInFlightPromise = null;

async function doPersistNotes() {
  try {
    const result = await window.callCommand('save_notes', {
      document: {
        schemaVersion: notesDocument.schemaVersion || 4,
        revision: notesDocument.revision || 0,
        updatedAt: new Date().toISOString(),
        content: '',
        meetingNotes: meetingArea.value,
        shortTermNotes: shortTermArea.value,
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
    window.showAppNotice(
      error.message || 'Could not save notes. Your draft is still here and will retry.',
      'danger',
      7000,
    );
    await window.refreshStorageStatus();
  } finally {
    notesSaveInFlightPromise = null;
  }
}

async function persistNotes() {
  if (!notesWritable || !notesDirty) return;

  // Serialize concurrent callers (debounced autosave vs. the quit-flush save hook)
  // instead of letting two save_notes round-trips race on the same read-modify-write.
  if (notesSaveInFlightPromise) {
    await notesSaveInFlightPromise.catch(() => {});
    return persistNotes();
  }

  notesSaveInFlightPromise = doPersistNotes();
  return notesSaveInFlightPromise;
}

const saveNotes = window.debounce(persistNotes, 350);

function markNotesDirty() {
  notesDirty = true;
  saveNotes();
}

meetingArea.addEventListener('input', markNotesDirty);
shortTermArea.addEventListener('input', markNotesDirty);
generalArea.addEventListener('input', markNotesDirty);

window.registerSaveHook(persistNotes);

// Expose current notes state for the archive feature.
window.getCurrentNotesForArchive = function getCurrentNotesForArchive() {
  return {
    meetingNotes: meetingArea.value,
    shortTermNotes: shortTermArea.value,
    generalNotes: generalArea.value,
  };
};

function setNotesEnabled(enabled) {
  const offlinePlaceholder = 'Notes are unavailable until the configured shared-data location is reachable.';
  const areas = [
    [meetingArea, 'Notes for your next meeting...'],
    [shortTermArea, 'Short-term notes...'],
    [generalArea, 'Long-term notes...'],
  ];
  areas.forEach(([area, placeholder]) => {
    area.disabled = !enabled;
    area.placeholder = enabled ? placeholder : offlinePlaceholder;
  });
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
