const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('api', {
  // File I/O
  loadFile: (filename) => ipcRenderer.invoke('load-file', filename),
  saveFile: (filename, data) => ipcRenderer.invoke('save-file', filename, data),

  // Shell
  openExternal: (url) => ipcRenderer.invoke('open-external', url),

  // Window controls
  minimize: () => ipcRenderer.invoke('window-minimize'),
  close: () => ipcRenderer.invoke('window-close'),
  toggleAlwaysOnTop: () => ipcRenderer.invoke('window-toggle-always-on-top'),

  // Desk365
  fetchTickets: (apiKey, desk365Domain) => ipcRenderer.invoke('fetch-tickets', apiKey, desk365Domain),

  // Notifications
  showNotification: (title, body) => ipcRenderer.invoke('show-notification', title, body),

  // Quick add
  quickAddTask: (title) => ipcRenderer.invoke('quick-add-task', title),
  closeQuickAdd: () => ipcRenderer.invoke('close-quick-add'),

  // Events from main process
  onTasksUpdated: (callback) => ipcRenderer.on('tasks-updated', callback),
});
