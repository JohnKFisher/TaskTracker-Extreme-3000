// === Notes Section ===

const notesArea = document.getElementById('notes-area');

// Load notes
async function loadNotes() {
  const data = await invoke('load_file', { filename: 'notes.json' });
  if (data && data.content !== undefined) {
    notesArea.value = data.content;
  }
}

// Save notes (debounced)
const saveNotes = debounce(async () => {
  await invoke('save_file', {
    filename: 'notes.json',
    data: {
      schemaVersion: 1,
      content: notesArea.value,
      updatedAt: new Date().toISOString(),
    },
  });
}, 500);

notesArea.addEventListener('input', saveNotes);

// Load on startup
loadNotes();
