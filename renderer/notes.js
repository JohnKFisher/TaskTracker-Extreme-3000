const notesArea = document.getElementById('notes-area');
let notesWritable = true;

async function loadNotes() {
  try {
    const data = await window.callCommand('load_notes');
    notesArea.value = data.content || '';
  } catch (error) {
    notesArea.value = '';
    console.error('Failed to load notes:', error);
  }
}

async function persistNotes() {
  if (!notesWritable) return;

  try {
    await window.callCommand('save_notes', {
      document: {
        schemaVersion: 1,
        content: notesArea.value,
        updatedAt: new Date().toISOString(),
      },
    });
  } catch (error) {
    console.error('Failed to save notes:', error);
    await window.refreshStorageStatus();
    await loadNotes();
  }
}

const saveNotes = window.debounce(persistNotes, 350);

notesArea.addEventListener('input', saveNotes);
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

loadNotes();
