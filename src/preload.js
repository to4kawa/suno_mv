const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  generateMP4: (params) => ipcRenderer.invoke('generate-mp4', params)
});
