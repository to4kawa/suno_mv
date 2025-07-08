const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');

// ハードコードされたFFmpegパス
const HARDCODED_FFMPEG_PATH = 'C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe';

function getConfigPath() {
    return path.join(__dirname, '../setting.json');
}

function loadConfig() {
    const configPath = getConfigPath();
    if (fs.existsSync(configPath)) {
        try {
            return JSON.parse(fs.readFileSync(configPath, 'utf-8'));
        } catch (e) {
            console.error('設定ファイルの読み込みに失敗:', e);
            return null;
        }
    }
    return null;
}

function createWindow() {
    const win = new BrowserWindow({
        width: 1080,
        height: 800,
        webPreferences: {
            preload: path.join(__dirname, 'preload.js'),
            contextIsolation: true,
            sandbox: true
        }
    });

    win.loadFile(path.join(__dirname, '../public/index.html'));

    // 設定取得
    ipcMain.handle('get-config', async () => {
        const cfg = loadConfig();
        
        // setting.jsonが存在しない or ハードコードされたFFmpegパスが有効
        if (!cfg || !cfg.ffmpegPath || !fs.existsSync(cfg.ffmpegPath)) {
            return {
                ffmpegPath: HARDCODED_FFMPEG_PATH,
                fallback: true
            };
        }
        
        return cfg;
    });
}

app.whenReady().then(createWindow);
app.on("window-all-closed", () => {
    if (process.platform !== "darwin") app.quit();
});
app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
});