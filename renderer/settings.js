async function initSettings() {
  try {
    const settings = await window.callCommand('load_local_settings_cmd');
    updateSyncFolderDisplay(settings.sync_folder || null);
  } catch (error) {
    console.error('Failed to load local settings:', error);
  }

  renderStorageStatus(window.currentStorageStatus);
  renderAbout(window.appMetadata);
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

function renderStorageStatus(status) {
  if (!status) return;

  const modeValue = document.getElementById('storage-mode-value');
  const message = document.getElementById('storage-message');

  if (status.mode === 'sync') {
    modeValue.textContent = `Using shared folder: ${status.activePath}`;
    message.classList.add('hidden');
    message.textContent = '';
  } else if (status.mode === 'syncUnavailable') {
    modeValue.textContent = `Configured shared folder unavailable: ${status.configuredPath}`;
    message.textContent = status.message || '';
    message.classList.remove('hidden');
  } else if (status.mode === 'localUnavailable') {
    modeValue.textContent = 'Local app data unavailable';
    message.textContent = status.message || '';
    message.classList.remove('hidden');
  } else {
    modeValue.textContent = `Using local app data: ${status.activePath}`;
    message.classList.add('hidden');
    message.textContent = '';
  }
}

function renderAbout(metadata) {
  if (!metadata) return;

  document.getElementById('about-product').textContent = metadata.productName;
  document.getElementById('about-version').textContent = `Version ${metadata.marketingVersion} (Build ${metadata.buildNumber})`;
  document.getElementById('about-copyright').textContent = metadata.copyright;
  document.getElementById('about-license').textContent = `License: ${metadata.license} • Primary platform: ${metadata.primaryPlatform}`;
}

document.getElementById('btn-browse-sync-folder').addEventListener('click', async () => {
  try {
    const folder = await window.callCommand('pick_sync_folder');
    if (!folder) return;

    const status = await window.callCommand('save_local_settings_cmd', {
      settings: { sync_folder: folder },
    });

    updateSyncFolderDisplay(folder);
    renderStorageStatus(status);
    await window.refreshStorageStatus();
  } catch (error) {
    console.error('Failed to save sync folder:', error);
  }
});

document.getElementById('btn-clear-sync-folder').addEventListener('click', async () => {
  const confirmed = confirm('Clear the sync folder? Future shared-data writes will stop going to the current shared folder and return to local app data.');
  if (!confirmed) return;

  try {
    const status = await window.callCommand('save_local_settings_cmd', {
      settings: { sync_folder: null },
    });

    updateSyncFolderDisplay(null);
    renderStorageStatus(status);
    await window.refreshStorageStatus();
  } catch (error) {
    console.error('Failed to clear sync folder:', error);
  }
});

document.getElementById('btn-open-github').addEventListener('click', async () => {
  if (!window.appMetadata) return;
  try {
    await window.openExternal(window.appMetadata.githubUrl);
  } catch (error) {
    console.error('Failed to open GitHub URL:', error);
  }
});

window.addEventListener('storage-status-changed', (event) => {
  renderStorageStatus(event.detail);
});

window.addEventListener('app-metadata-changed', (event) => {
  renderAbout(event.detail);
});

initSettings();
