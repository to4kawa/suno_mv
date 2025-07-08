// preload.js 修正版
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    generateMP4WithSunoCover: (params) => ipcRenderer.invoke('generate-mp4-with-suno-cover', params),
    generateMP4WithBase64: (params) => ipcRenderer.invoke('generate-mp4-with-base64', params),
    openSettingsWindow: () => ipcRenderer.send('open-settings-window')
});