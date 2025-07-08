const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    generateMP4WithSunoCover: (params) => ipcRenderer.invoke('generate-mp4-suno', params),
    generateMP4WithBase64:    (params) => ipcRenderer.invoke('generate-mp4-base64', params),
    getFFmpegPath:            ()      => ipcRenderer.invoke('get-ffmpeg-path'),
    saveFFmpegPath:           (path)  => ipcRenderer.invoke('save-ffmpeg-path', path)
});
