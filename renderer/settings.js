// === Settings Panel ===

async function initSettings() {
  const settings = await invoke('load_local_settings_cmd');
  updateSyncFolderDisplay(settings.sync_folder || null);
}

function updateSyncFolderDisplay(folder) {
  const display = document.getElementById('sync-folder-display');
  const clearBtn = document.getElementById('btn-clear-sync-folder');

  if (folder) {
    display.textContent = folder;
    display.title = folder;
    clearBtn.classList.remove('hidden');
  } else {
    display.textContent = 'None — using local app data folder';
    display.title = '';
    clearBtn.classList.add('hidden');
  }
}

document.getElementById('btn-browse-sync-folder').addEventListener('click', async () => {
  const folder = await invoke('pick_sync_folder');
  if (folder) {
    const saved = await invoke('save_local_settings_cmd', {
      settings: { sync_folder: folder },
    });
    if (saved) {
      updateSyncFolderDisplay(folder);
    }
  }
});

document.getElementById('btn-clear-sync-folder').addEventListener('click', async () => {
  await invoke('save_local_settings_cmd', { settings: { sync_folder: null } });
  updateSyncFolderDisplay(null);
});

initSettings();
