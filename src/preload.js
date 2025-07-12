const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    generateMP4WithSunoCover: (params) => ipcRenderer.invoke('generate-mp4-with-suno-cover', params),
    generateMP4WithBase64: (params) => ipcRenderer.invoke('generate-mp4-with-base64', params),
    selectOutputDir: () => ipcRenderer.invoke('select-output-dir'),
    getOutputDir: () => ipcRenderer.invoke('get-output-dir')
});
