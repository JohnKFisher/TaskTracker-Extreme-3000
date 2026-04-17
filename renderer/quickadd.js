const invoke = window.__TAURI__.core.invoke;

async function callCommand(command, args = {}) {
  const result = await invoke(command, args);
  if (!result.success) {
    throw result.error || { code: 'unknown', message: 'Unknown error.' };
  }
  return result.data;
}

const input = document.getElementById('quick-input');
const status = document.getElementById('quick-add-status');

input.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    await callCommand('close_quick_add');
    return;
  }

  if (event.key !== 'Enter') return;

  const title = input.value.trim().replace(/^./, (c) => c.toUpperCase());
  if (!title) {
    await callCommand('close_quick_add');
    return;
  }

  try {
    status.textContent = '';
    await callCommand('quick_add_task', { title });
  } catch (error) {
    status.textContent = error.message || 'Could not add the task.';
  }
});
