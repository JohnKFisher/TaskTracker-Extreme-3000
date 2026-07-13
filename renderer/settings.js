let currentLocalSettings = null;

async function initSettings() {
  await refreshLocalSettingsDisplay();

  renderStorageStatus(window.currentStorageStatus);
  renderSyncFolderNotice(null);
  renderAbout(window.appMetadata);
}

async function loadCurrentLocalSettings() {
  currentLocalSettings = await window.callCommand('load_local_settings_cmd');
  return currentLocalSettings;
}

function applyLocalSettings(settings) {
  currentLocalSettings = settings;
  updateSyncFolderDisplay(settings.syncFolder || null);
  updatePersonalTabToggle(Boolean(settings.showPersonalTab));
  if (typeof window.applyPersonalTabVisibility === 'function') {
    window.applyPersonalTabVisibility(Boolean(settings.showPersonalTab));
  }
  const showStanding = settings.showStandingColumn !== false;
  updateStandingColumnToggle(showStanding);
  if (typeof window.applyStandingColumnVisibility === 'function') {
    window.applyStandingColumnVisibility(showStanding);
  }
  updateColorThemeToggle(settings.colorTheme || 'auto');
  window.applyColorTheme(settings.colorTheme || 'auto');
  updateGcsDisplay(settings.gcsCredentialPath || null, settings.gcsBucket || null);
}

function updateColorThemeToggle(theme) {
  document.querySelectorAll('input[name="color-theme"]').forEach((radio) => {
    radio.checked = radio.value === theme;
  });
}

async function refreshLocalSettingsDisplay() {
  try {
    const settings = await loadCurrentLocalSettings();
    applyLocalSettings(settings);
  } catch (error) {
    console.error('Failed to load local settings:', error);
    renderSyncFolderNotice(error.message || 'Could not load local settings.', 'danger');
  }
}

async function saveLocalSettingsPatch(patch) {
  const current = currentLocalSettings || await loadCurrentLocalSettings();
  const nextSettings = {
    ...current,
    ...patch,
  };

  const status = await window.callCommand('save_local_settings_cmd', {
    settings: nextSettings,
  });

  applyLocalSettings(nextSettings);
  return status;
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

function updatePersonalTabToggle(visible) {
  const toggle = document.getElementById('toggle-personal-tab');
  toggle.checked = visible;
}

function updateStandingColumnToggle(visible) {
  const toggle = document.getElementById('toggle-standing-column');
  toggle.checked = visible;
}

function renderStorageStatus(status) {
  if (!status) return;

  const modeValue = document.getElementById('storage-mode-value');
  const message = document.getElementById('storage-message');

  if (status.mode === 'gcs') {
    modeValue.textContent = `Using GCS bucket: ${status.activePath}`;
    message.classList.add('hidden');
    message.textContent = '';
  } else if (status.mode === 'sync') {
    modeValue.textContent = `Using shared folder: ${status.activePath}`;
    message.classList.add('hidden');
    message.textContent = '';
  } else if (status.mode === 'syncUnavailable') {
    modeValue.textContent = `Configured shared folder unavailable: ${status.configuredPath}`;
    message.textContent = status.message || '';
    message.classList.remove('hidden');
  } else if (status.mode === 'gcsUnavailable') {
    modeValue.textContent = `Configured GCS bucket unavailable: ${status.configuredPath}`;
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

function updateGcsDisplay(credentialPath, bucket) {
  const credentialInput = document.getElementById('gcs-credential-path');
  const bucketInput = document.getElementById('gcs-bucket');
  const migrateBtn = document.getElementById('btn-migrate-gcs');
  const clearBtn = document.getElementById('btn-clear-gcs');

  credentialInput.value = credentialPath || '';
  bucketInput.value = bucket || '';

  const isActive = Boolean(credentialPath && bucket);
  // Migrating only makes sense once GCS is actually reachable — offering it while
  // the bucket/credentials are unconfirmed just invites migrating into a dead end.
  const isConnected = isActive
    && window.currentStorageStatus
    && window.currentStorageStatus.mode === 'gcs';
  migrateBtn.classList.toggle('hidden', !isConnected);
  clearBtn.classList.toggle('hidden', !isActive);
}

function renderGcsResult(message, tone = 'info') {
  const el = document.getElementById('gcs-result');
  if (!message) {
    el.classList.add('hidden');
    el.classList.remove('info', 'warning', 'danger');
    el.textContent = '';
    return;
  }
  el.textContent = message;
  el.classList.remove('hidden', 'info', 'warning', 'danger');
  el.classList.add(tone);
}

function renderSyncFolderNotice(message, tone = 'info') {
  const notice = document.getElementById('sync-folder-result');
  if (!message) {
    notice.classList.add('hidden');
    notice.classList.remove('info', 'warning', 'danger');
    notice.textContent = '';
    return;
  }

  notice.textContent = message;
  notice.classList.remove('hidden');
  notice.classList.remove('info', 'warning', 'danger');
  notice.classList.add(tone);
}

function renderAbout(metadata) {
  if (!metadata) return;

  document.getElementById('about-product').textContent = metadata.productName;
  document.getElementById('about-version').textContent = `Version ${metadata.marketingVersion} (Build ${metadata.buildNumber})`;
  document.getElementById('about-copyright').textContent = metadata.copyright;
  document.getElementById('about-license').textContent = `License: ${metadata.license} • Primary platform: ${metadata.primaryPlatform}`;
}

async function applyStorageStatusResult(status) {
  await refreshLocalSettingsDisplay();
  renderStorageStatus(status);
  const noticeTone = status.noticeTone || 'info';
  renderSyncFolderNotice(status.notice || null, noticeTone);
  if (status.notice) {
    window.showAppNotice(status.notice, noticeTone, 9000);
  }
  await window.refreshStorageStatus();
}

document.getElementById('btn-browse-sync-folder').addEventListener('click', async () => {
  try {
    const folder = await window.callCommand('pick_sync_folder');
    if (!folder) return;

    const status = await saveLocalSettingsPatch({ syncFolder: folder });
    updateSyncFolderDisplay(folder);
    await applyStorageStatusResult(status);
  } catch (error) {
    console.error('Failed to save sync folder:', error);
    renderSyncFolderNotice(error.message || 'Could not save the sync folder.', 'danger');
  }
});

document.getElementById('btn-clear-sync-folder').addEventListener('click', async () => {
  // GCS takes priority over the sync folder when both are configured, so clearing
  // the folder alone won't move active storage back to local app data in that case.
  const gcsConfigured = Boolean(currentLocalSettings && currentLocalSettings.gcsCredentialPath && currentLocalSettings.gcsBucket);
  const confirmMessage = gcsConfigured
    ? 'Clear the sync folder? GCS sync is also configured and takes priority, so shared data will keep going to GCS — this only clears the unused folder setting.'
    : 'Clear the sync folder? Future shared-data writes will stop going to the current shared folder and return to local app data.';
  const confirmed = confirm(confirmMessage);
  if (!confirmed) return;

  try {
    const status = await saveLocalSettingsPatch({ syncFolder: null });
    updateSyncFolderDisplay(null);
    await applyStorageStatusResult(status);
  } catch (error) {
    console.error('Failed to clear sync folder:', error);
    renderSyncFolderNotice(error.message || 'Could not clear the sync folder.', 'danger');
  }
});

document.querySelectorAll('input[name="color-theme"]').forEach((radio) => {
  radio.addEventListener('change', async (event) => {
    if (!event.target.checked) return;
    try {
      await saveLocalSettingsPatch({ colorTheme: event.target.value });
    } catch (error) {
      console.error('Failed to save color theme:', error);
      updateColorThemeToggle(currentLocalSettings?.colorTheme || 'auto');
    }
  });
});

document.getElementById('toggle-standing-column').addEventListener('change', async (event) => {
  try {
    await saveLocalSettingsPatch({ showStandingColumn: event.target.checked });
  } catch (error) {
    console.error('Failed to save Standing column visibility:', error);
    event.target.checked = !event.target.checked;
    renderSyncFolderNotice(error.message || 'Could not update the Standing column setting.', 'danger');
  }
});

document.getElementById('toggle-personal-tab').addEventListener('change', async (event) => {
  try {
    await saveLocalSettingsPatch({ showPersonalTab: event.target.checked });
  } catch (error) {
    console.error('Failed to save personal tab visibility:', error);
    event.target.checked = !event.target.checked;
    renderSyncFolderNotice(error.message || 'Could not update the Personal tab setting.', 'danger');
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

document.getElementById('btn-browse-gcs-credential').addEventListener('click', async () => {
  try {
    const result = await window.__TAURI__.dialog.open({
      filters: [{ name: 'JSON Key', extensions: ['json'] }],
      multiple: false,
    });
    if (!result) return;
    const path = typeof result === 'string' ? result : result.path ?? result;
    document.getElementById('gcs-credential-path').value = path;
    renderGcsResult(null);
  } catch (error) {
    renderGcsResult(error.message || 'Could not open file picker.', 'danger');
  }
});

document.getElementById('btn-test-gcs').addEventListener('click', async () => {
  const credentialPath = document.getElementById('gcs-credential-path').value.trim();
  const bucket = document.getElementById('gcs-bucket').value.trim();
  if (!credentialPath || !bucket) {
    renderGcsResult('Enter both a credential file path and a bucket name before testing.', 'warning');
    return;
  }
  renderGcsResult('Testing connection…');
  try {
    const result = await window.callCommand('test_gcs_connection', { credentialPath, bucket });
    renderGcsResult(result, 'info');
  } catch (error) {
    renderGcsResult(error.message || 'Connection test failed.', 'danger');
  }
});

document.getElementById('btn-save-gcs').addEventListener('click', async () => {
  const credentialPath = document.getElementById('gcs-credential-path').value.trim();
  const bucket = document.getElementById('gcs-bucket').value.trim();
  if (!credentialPath || !bucket) {
    renderGcsResult('Enter both a credential file path and a bucket name.', 'warning');
    return;
  }
  try {
    const status = await saveLocalSettingsPatch({ gcsCredentialPath: credentialPath, gcsBucket: bucket });
    updateGcsDisplay(credentialPath, bucket);
    if (status.mode === 'gcs') {
      renderGcsResult(`GCS sync enabled for bucket "${bucket}". Existing files will load from GCS on next sync.`, 'info');
    } else {
      renderGcsResult(
        status.message || `Saved, but could not connect to bucket "${bucket}". Check the credential file and bucket name.`,
        'danger',
      );
    }
    renderStorageStatus(status);
    await window.refreshStorageStatus();
  } catch (error) {
    renderGcsResult(error.message || 'Could not save GCS config.', 'danger');
  }
});

document.getElementById('btn-migrate-gcs').addEventListener('click', async () => {
  const confirmed = confirm(
    'This will copy your current local/sync-folder tasks, notes, and settings into GCS. Any existing GCS data will be overwritten. Continue?'
  );
  if (!confirmed) return;
  renderGcsResult('Migrating files to GCS…');
  try {
    const result = await window.callCommand('migrate_to_gcs');
    renderGcsResult(result, 'info');
  } catch (error) {
    renderGcsResult(error.message || 'Migration failed.', 'danger');
  }
});

document.getElementById('btn-clear-gcs').addEventListener('click', async () => {
  const confirmed = confirm(
    'Disable GCS sync? The app will fall back to local or sync-folder storage. Your data in GCS will not be deleted.'
  );
  if (!confirmed) return;
  try {
    const status = await saveLocalSettingsPatch({ gcsCredentialPath: null, gcsBucket: null });
    updateGcsDisplay(null, null);
    renderGcsResult('GCS sync disabled. Falling back to local/sync-folder storage.', 'info');
    renderStorageStatus(status);
    await window.refreshStorageStatus();
  } catch (error) {
    renderGcsResult(error.message || 'Could not disable GCS sync.', 'danger');
  }
});

window.addEventListener('storage-status-changed', (event) => {
  renderStorageStatus(event.detail);
  if (currentLocalSettings) {
    updateGcsDisplay(currentLocalSettings.gcsCredentialPath || null, currentLocalSettings.gcsBucket || null);
  }
});

window.addEventListener('app-metadata-changed', (event) => {
  renderAbout(event.detail);
});

initSettings();
