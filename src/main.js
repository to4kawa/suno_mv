const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');

function getConfigPath() {
  // プロジェクト直下の場合
  return path.join(__dirname, '../setting.json');
}

function loadConfig() {
  const configPath = getConfigPath();
  if (fs.existsSync(configPath)) {
    try {
      return JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    } catch (e) {
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
    }
  });

  win.loadFile(path.join(__dirname, '../public/index.html'));

  // 設定リクエスト
  ipcMain.handle('get-config', async () => {
    const cfg = loadConfig();
    if (!cfg || !cfg.ffmpegPath || !fs.existsSync(cfg.ffmpegPath)) {
      return { error: 'setting.jsonが存在しない、またはffmpegパスが無効です。' };
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