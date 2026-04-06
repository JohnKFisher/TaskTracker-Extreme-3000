const { app, BrowserWindow, ipcMain, shell, screen, globalShortcut, Tray, Menu, Notification, nativeImage } = require('electron');
const path = require('path');
const fs = require('fs');

// Data directory — use Electron's userData folder when packaged, local data/ when in dev
const DATA_DIR = app.isPackaged
  ? app.getPath('userData')
  : path.join(__dirname, 'data');

// Ensure data directory exists
if (!fs.existsSync(DATA_DIR)) {
  fs.mkdirSync(DATA_DIR, { recursive: true });
}

// Migrate data from the old hardcoded OneDrive location (one-time, safe to remove after v2)
function migrateDataIfNeeded() {
  const oldDataDir = path.join(
    process.env.USERPROFILE || process.env.HOME,
    'OneDrive - VNANNJ',
    "John's TaskTracker",
    'data'
  );
  if (!fs.existsSync(oldDataDir)) return;
  if (fs.existsSync(path.join(DATA_DIR, 'tasks.json'))) return; // already migrated

  try {
    const files = fs.readdirSync(oldDataDir).filter(f => f.endsWith('.json'));
    for (const file of files) {
      fs.copyFileSync(path.join(oldDataDir, file), path.join(DATA_DIR, file));
    }
    console.log(`Migrated ${files.length} data files from old location.`);
  } catch (err) {
    console.warn('Migration failed:', err.message);
  }
}

let appbar;
try {
  appbar = require('./appbar');
} catch (err) {
  console.warn('AppBar not available:', err.message);
  appbar = null;
}

let mainWindow = null;
let tray = null;
let quickAddWindow = null;

// Single-instance lock — prevent multiple copies of the app
const gotTheLock = app.requestSingleInstanceLock();

if (!gotTheLock) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore();
      mainWindow.show();
      mainWindow.focus();
    }
  });
}

// Window state persistence
function loadWindowState() {
  const stateFile = path.join(DATA_DIR, 'window-state.json');
  try {
    return JSON.parse(fs.readFileSync(stateFile, 'utf-8'));
  } catch {
    return null;
  }
}

function saveWindowState() {
  if (!mainWindow) return;
  const bounds = mainWindow.getBounds();
  const stateFile = path.join(DATA_DIR, 'window-state.json');
  try {
    fs.writeFileSync(stateFile, JSON.stringify(bounds, null, 2));
  } catch { /* ignore write errors */ }
}

function isOnScreen(x, y, width, height) {
  return screen.getAllDisplays().some(d => {
    const b = d.workArea;
    return x < b.x + b.width && x + width > b.x &&
           y < b.y + b.height && y + height > b.y;
  });
}

function createWindow() {
  const primaryDisplay = screen.getPrimaryDisplay();
  const { width: screenWidth, height: screenHeight } = primaryDisplay.workAreaSize;

  const savedState = loadWindowState();
  const winWidth = savedState?.width || 380;
  const winHeight = savedState?.height || screenHeight;
  let winX = savedState?.x ?? (screenWidth - winWidth);
  let winY = savedState?.y ?? 0;

  // If saved position is off all current displays, reset to primary display default
  if (savedState && !isOnScreen(winX, winY, winWidth, winHeight)) {
    winX = screenWidth - winWidth;
    winY = 0;
  }

  mainWindow = new BrowserWindow({
    width: winWidth,
    height: winHeight,
    x: winX,
    y: winY,
    alwaysOnTop: true,
    frame: false,
    resizable: true,
    skipTaskbar: false,
    transparent: false,
    backgroundColor: '#1a1a2e',
    minWidth: 300,
    maxWidth: 600,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  mainWindow.loadFile(path.join(__dirname, 'renderer', 'index.html'));

  // Transparency when unfocused
  mainWindow.on('blur', () => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.setOpacity(0.7);
    }
  });

  mainWindow.on('focus', () => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.setOpacity(1.0);
    }
  });

  // Save window state on move/resize (NO appbar update — fixed reservation only)
  mainWindow.on('moved', () => saveWindowState());
  mainWindow.on('resized', () => saveWindowState());

  mainWindow.on('close', (e) => {
    // Minimize to tray instead of closing
    e.preventDefault();
    mainWindow.hide();
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

function createTray() {
  // Create a simple tray icon (16x16 blue square)
  const icon = nativeImage.createFromBuffer(
    Buffer.from(
      'iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAKklEQVQ4y2NgoBb4z8BAPWBkYGD4T61' +
      'gFBgNg9EwGA2D0TAYDYPRMAAA3pAEEXfcoHoAAAAASUVORK5CYII=',
      'base64'
    )
  );
  tray = new Tray(icon);
  tray.setToolTip('TaskTracker Extreme 3000');

  const contextMenu = Menu.buildFromTemplate([
    { label: 'Show/Hide', click: () => toggleWindow() },
    { type: 'separator' },
    { label: 'Quit', click: () => { mainWindow.destroy(); app.quit(); } }
  ]);
  tray.setContextMenu(contextMenu);
  tray.on('click', () => toggleWindow());
}

function toggleWindow() {
  if (!mainWindow) return;
  if (mainWindow.isVisible()) {
    if (appbar) {
      try { appbar.releaseReserve(mainWindow); } catch { /* ignore */ }
    }
    mainWindow.hide();
  } else {
    mainWindow.show();
    mainWindow.focus();
    if (appbar) {
      const display = screen.getPrimaryDisplay();
      const appWidth = mainWindow.getBounds().width;
      try { appbar.reserveRight(mainWindow, display.size.width, display.size.height, appWidth); } catch { /* ignore */ }
    }
  }
}

function createQuickAddWindow() {
  if (quickAddWindow && !quickAddWindow.isDestroyed()) {
    quickAddWindow.focus();
    return;
  }

  const primaryDisplay = screen.getPrimaryDisplay();
  const { width: screenWidth } = primaryDisplay.workAreaSize;

  quickAddWindow = new BrowserWindow({
    width: 400,
    height: 80,
    x: Math.round((screenWidth - 400) / 2),
    y: 100,
    frame: false,
    alwaysOnTop: true,
    resizable: false,
    skipTaskbar: true,
    transparent: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  quickAddWindow.loadFile(path.join(__dirname, 'renderer', 'quickadd.html'));

  quickAddWindow.on('blur', () => {
    if (quickAddWindow && !quickAddWindow.isDestroyed()) {
      quickAddWindow.close();
    }
  });

  quickAddWindow.on('closed', () => {
    quickAddWindow = null;
  });
}

// IPC Handlers — file I/O
ipcMain.handle('load-file', async (event, filename) => {
  const filePath = path.join(DATA_DIR, filename);
  try {
    const data = fs.readFileSync(filePath, 'utf-8');
    return JSON.parse(data);
  } catch {
    return null;
  }
});

ipcMain.handle('save-file', async (event, filename, data) => {
  const filePath = path.join(DATA_DIR, filename);
  try {
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
    return true;
  } catch (err) {
    console.error('Save error:', err);
    return false;
  }
});

ipcMain.handle('open-external', async (event, url) => {
  await shell.openExternal(url);
});

// Window control IPC
ipcMain.handle('window-minimize', () => {
  if (mainWindow) mainWindow.minimize();
});

ipcMain.handle('window-close', () => {
  if (mainWindow) mainWindow.hide();
});

ipcMain.handle('window-toggle-always-on-top', () => {
  if (mainWindow) {
    const current = mainWindow.isAlwaysOnTop();
    mainWindow.setAlwaysOnTop(!current);
    return !current;
  }
  return true;
});

// Quick add IPC
ipcMain.handle('quick-add-task', async (event, title) => {
  // Load tasks, add new one, save
  const filePath = path.join(DATA_DIR, 'tasks.json');
  let data;
  try {
    data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  } catch {
    data = { schemaVersion: 1, tasks: [] };
  }

  const task = {
    id: 't_' + Date.now(),
    title: title,
    notes: '',
    column: 'todo',
    order: 0,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  // Shift existing to-do tasks down
  data.tasks.forEach(t => {
    if (t.column === 'todo') t.order++;
  });
  data.tasks.push(task);

  fs.writeFileSync(filePath, JSON.stringify(data, null, 2));

  // Notify main window to refresh
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('tasks-updated');
  }

  // Close quick add window
  if (quickAddWindow && !quickAddWindow.isDestroyed()) {
    quickAddWindow.close();
  }

  return true;
});

ipcMain.handle('close-quick-add', () => {
  if (quickAddWindow && !quickAddWindow.isDestroyed()) {
    quickAddWindow.close();
  }
});

// Desk365 API fetch (done in main process to avoid CORS)
ipcMain.handle('fetch-tickets', async (event, apiKey, desk365Domain) => {
  const baseUrl = `https://${desk365Domain}/apis/v3/tickets`;
  const allTickets = [];
  let offset = 0;
  const batchSize = 30;

  try {
    // Fetch pages until we have all tickets
    while (true) {
      const url = `${baseUrl}?offset=${offset}&order_by=updated_time&order_type=descending`;
      const response = await fetch(url, {
        headers: {
          'Authorization': apiKey,
          'Accept': 'application/json',
        },
      });

      if (!response.ok) {
        let errorDetail = response.statusText;
        try {
          const errorBody = await response.text();
          if (errorBody) errorDetail += ' — ' + errorBody.substring(0, 200);
        } catch { /* ignore */ }
        throw new Error(`API error: ${response.status} ${errorDetail}`);
      }

      const result = await response.json();
      const tickets = result.tickets || result.data || [];

      // Debug: log first ticket's fields so we can see the actual API shape
      if (offset === 0 && tickets.length > 0) {
        console.log('Desk365 sample ticket keys:', Object.keys(tickets[0]));
        console.log('Desk365 sample ticket:', JSON.stringify(tickets[0]).substring(0, 500));
      }

      allTickets.push(...tickets);

      // If we got fewer than batchSize, we've reached the end
      if (tickets.length < batchSize) break;
      offset += batchSize;

      // Safety limit — don't fetch more than 300 tickets
      if (offset >= 300) break;

      // Rate limit: Desk365 allows max 2 req/sec
      await new Promise(resolve => setTimeout(resolve, 600));
    }

    // Normalize field names — Desk365 API may use snake_case or PascalCase
    const normalized = allTickets.map(t => ({
      TicketNumber: t.TicketNumber || t.ticket_number || t.ticketNumber || t.id,
      TicketId: t.ticket_id || t.TicketId || t.id || t.ticket_number || t.TicketNumber,
      Subject: t.Subject || t.subject || t.title || '(no subject)',
      Status: t.Status || t.status || t.ticket_status || 'Unknown',
      Priority: t.Priority || t.priority || '',
      Agent: t.Agent || t.agent || t.assigned_to || t.assignee || '',
      Category: t.Category || t.category || '',
      UpdatedAt: t.UpdatedAt || t.updated_time || t.updated_at || '',
      _raw: t, // keep raw data for debugging
    }));

    // Filter to unresolved only
    const unresolved = normalized.filter(t =>
      !['Closed', 'Resolved', 'closed', 'resolved'].includes(t.Status)
    );

    return { success: true, tickets: unresolved, total: allTickets.length };
  } catch (err) {
    return { success: false, error: err.message };
  }
});

// Show notification
ipcMain.handle('show-notification', async (event, title, body) => {
  if (Notification.isSupported()) {
    const notification = new Notification({ title, body });
    notification.on('click', () => {
      if (mainWindow) {
        mainWindow.show();
        mainWindow.focus();
      }
    });
    notification.show();
  }
});

// App lifecycle
app.whenReady().then(() => {
  migrateDataIfNeeded();
  createWindow();
  createTray();

  // Reserve screen space on the right so maximized windows don't overlap
  if (appbar) {
    const display = screen.getPrimaryDisplay();
    const appWidth = mainWindow.getBounds().width;
    try {
      appbar.reserveRight(mainWindow, display.size.width, display.size.height, appWidth);
    } catch (err) {
      console.warn('Failed to reserve screen space:', err.message);
    }
  }

  // Global shortcuts
  globalShortcut.register('CommandOrControl+Shift+T', toggleWindow);
  globalShortcut.register('CommandOrControl+Shift+N', createQuickAddWindow);
});

app.on('will-quit', () => {
  if (appbar) {
    try { appbar.releaseReserve(mainWindow); } catch { /* ignore */ }
  }
  globalShortcut.unregisterAll();
});

app.on('window-all-closed', () => {
  // Don't quit — we have a tray icon
});

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow();
  }
});
