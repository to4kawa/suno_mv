const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    generateMP4WithBase64: (params) => ipcRenderer.invoke('generate-mp4-with-base64', params),
    getFFmpegPath: () => ipcRenderer.invoke('get-ffmpeg-path'),
    saveFFmpegPath: (newPath) => ipcRenderer.invoke('save-ffmpeg-path', newPath)
});