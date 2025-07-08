// ===============================
// main.js（Electronメインプロセス）
// ===============================

const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs');
const fsExtra = require('fs-extra'); // fs-extra: 必要なら使う
const { spawn } = require('child_process');

// ---- 固定パス設定 ----
const HARDCODED_FFMPEG_PATH = 'C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe';

// ---- 設定ファイル管理 ----
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

// ---- メインウィンドウ生成 ----
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

    // 設定取得（UI用）
    ipcMain.handle('get-config', async () => {
        const cfg = loadConfig();
        // setting.jsonがなければハードコード
        if (!cfg || !cfg.ffmpegPath || !fs.existsSync(cfg.ffmpegPath)) {
            return {
                ffmpegPath: HARDCODED_FFMPEG_PATH,
                fallback: true
            };
        }
        return cfg;
    });

    // ファイル選択ダイアログ
    ipcMain.handle('select-image', async () => {
        const result = await dialog.showOpenDialog({
            properties: ['openFile'],
            filters: [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png'] }]
        });
        if (result.canceled || !result.filePaths[0]) return null;
        return result.filePaths[0];
    });

    // MP4生成リクエスト
    ipcMain.handle('generate-mp4', async (event, { url, imagePath }) => {
        try {
            // ユーザーデータ保存先を取得
            const appPath = app.getPath('userData');
            const outputVideoPath = path.join(appPath, 'output.mp4');

            // 画像の存在チェック
            if (!imagePath || !fs.existsSync(imagePath)) {
                return { success: false, stderr: '画像ファイルが無効です' };
            }

            // ffmpeg.exeの存在チェック
            if (!fs.existsSync(HARDCODED_FFMPEG_PATH)) {
                return { success: false, stderr: 'ffmpeg.exe が見つかりません' };
            }

            // --- FFmpegコマンド ---
            const ffmpegArgs = [
                '-loop', '1', '-i', imagePath,
                '-i', 'audio.mp3', // ここは動的ならパスを渡す
                '-c:v', 'libx264', '-preset', 'fast',
                '-pix_fmt', 'yuv420p',
                '-c:a', 'aac', '-shortest',
                '-movflags', '+faststart',
                outputVideoPath
            ];

            // --- 実行 ---
            const ffmpeg = spawn(HARDCODED_FFMPEG_PATH, ffmpegArgs);
            let stdout = '', stderr = '';
            ffmpeg.stdout.on('data', data => stdout += data.toString());
            ffmpeg.stderr.on('data', data => stderr += data.toString());

            return await new Promise((resolve) => {
                ffmpeg.on('close', (code) => {
                    if (code === 0) {
                        resolve({ success: true, stdout, outputVideoPath });
                    } else {
                        resolve({ success: false, stderr });
                    }
                });
            });
        } catch (error) {
            return { success: false, error: error.message };
        }
    });
}

// ---- Electronアプリライフサイクル ----
app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
    if (process.platform !== "darwin") app.quit();
});

app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
});
