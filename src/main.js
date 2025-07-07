const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const { exec } = require('child_process');

function createWindow() {
  const win = new BrowserWindow({
    width: 1080,
    height: 800,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: true,
      contextIsolation: false
    }
  });

  win.loadFile(path.join(__dirname, '../public/index.html'));
  
  // メインで生成リクエストを受け、nodeスクリプトを実行
  ipcMain.handle('generate-mp4', async (event, { url, imagePath }) => {
    const scriptPath = path.join(__dirname, 'suno_mv.js');
    let cmd = `node "${scriptPath}" "${url}" "./output" "1280x720" "spectrum"`;
    if (imagePath) cmd += ` "${imagePath}"`;
    return new Promise((resolve, reject) => {
      exec(cmd, (err, stdout, stderr) => {
        if (err) {
          // stdout, stderr も必ず返す
          resolve({ success: false, stdout, stderr });
        } else {
          resolve({ success: true, stdout, stderr });
        }
      });
    });
  });
}

app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
    if (process.platform !== "darwin") app.quit();
});
