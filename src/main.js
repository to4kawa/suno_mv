const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { spawn } = require('child_process'); // 🔥 spawnを追加

// ハードコードされたFFmpegパス
const HARDCODED_FFMPEG_PATH = 'C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe';

// IPC: MP4生成（base64画像使用）
ipcMain.handle('generate-mp4-with-base64', async (event, { url, base64 }) => {
    try {
        const appPath = app.getPath('userData');
        const tempImagePath = path.join(appPath, 'temp_cover.jpg');
        const outputVideoPath = path.join(appPath, 'output.mp4');

        // 一時ディレクトリが存在しない場合は作成
        if (!fs.existsSync(appPath)) {
            fs.mkdirSync(appPath, { recursive: true });
        }

        // base64データを一時保存
        if (base64 && base64.startsWith('data:image')) {
            const base64Data = base64.replace(/^data:image\/jpeg;base64,/, '');
            fs.writeFileSync(tempImagePath, Buffer.from(base64Data, 'base64'));
        } else {
            return { success: false, stderr: '画像データがありません' };
        }

        // FFmpegコマンドの構築
        const ffmpegArgs = [
            '-loop', '1', '-i', tempImagePath,
            '-i', 'audio.mp3',
            '-c:v', 'libx264', '-preset', 'fast',
            '-pix_fmt', 'yuv420p',
            '-c:a', 'aac', '-shortest',
            '-movflags', '+faststart',
            outputVideoPath
        ];

        // FFmpeg実行
        const ffmpeg = spawn(HARDCODED_FFMPEG_PATH, ffmpegArgs);

        let stdout = '', stderr = '';
        ffmpeg.stdout.on('data', (data) => stdout += data.toString());
        ffmpeg.stderr.on('data', (data) => stderr += data.toString());

        // 🔍 spawnのエラーハンドリング追加
        ffmpeg.on('error', (err) => {
            console.error('FFmpeg実行エラー:', err);
        });

        return new Promise((resolve, reject) => {
            ffmpeg.on('close', (code) => {
                if (code === 0) {
                    resolve({ success: true, stdout });
                } else {
                    reject({ success: false, stderr });
                }
            });
        });
    } catch (error) {
        return { success: false, error: error.message };
    }
});

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