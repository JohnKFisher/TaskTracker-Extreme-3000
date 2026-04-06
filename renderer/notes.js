// === Notes Section ===

const notesArea = document.getElementById('notes-area');

// Load notes
async function loadNotes() {
  const data = await window.api.loadFile('notes.json');
  if (data && data.content !== undefined) {
    notesArea.value = data.content;
  }
}

// Save notes (debounced)
const saveNotes = debounce(async () => {
  await window.api.saveFile('notes.json', {
    schemaVersion: 1,
    content: notesArea.value,
    updatedAt: new Date().toISOString(),
  });
}, 500);

notesArea.addEventListener('input', saveNotes);

// Load on startup
loadNotes();
