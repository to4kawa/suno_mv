const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs');
const fsExtra = require('fs-extra');
const { spawn } = require('child_process');

const HARDCODED_FFMPEG_PATH = 'C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe';

// ハンドラ名を統一
ipcMain.handle('generate-mp4-with-suno-cover', async (event, { url }) => {
    try {
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) return { success: false, stderr: 'Suno曲のURLが無効です' };

        const id = m[1];
        const appPath = app.getPath('userData');
        const tempImagePath = path.join(appPath, 'temp_cover.jpg');
        const outputVideoPath = path.join(appPath, 'output.mp4');

        // 一時ディレクトリがなければ作成
        if (!fs.existsSync(appPath)) fs.mkdirSync(appPath, { recursive: true });

        // Suno画像をダウンロード
        const imageResponse = await fetch(`https://cdn2.suno.ai/image_large_${id}.jpeg`);
        const imageBuffer = Buffer.from(await imageResponse.arrayBuffer());
        fs.writeFileSync(tempImagePath, imageBuffer);

        // FFmpegコマンド
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
        ffmpeg.stdout.on('data', data => stdout += data.toString());
        ffmpeg.stderr.on('data', data => stderr += data.toString());

        return new Promise((resolve, reject) => {
            ffmpeg.on('close', (code) => {
                if (code === 0) resolve({ success: true, stdout });
                else reject({ success: false, stderr });
            });
        });
    } catch (error) {
        return { success: false, error: error.message };
    }
});

// base64画像を処理するハンドラ
ipcMain.handle('generate-mp4-with-base64', async (event, { url, base64 }) => {
    try {
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) return { success: false, stderr: 'Suno曲のURLが無効です' };

        const appPath = app.getPath('userData');
        const tempImagePath = path.join(appPath, 'temp_cover.jpg');
        const outputVideoPath = path.join(appPath, 'output.mp4');

        // 一時ディレクトリがなければ作成
        if (!fs.existsSync(appPath)) fs.mkdirSync(appPath, { recursive: true });

        // base64データを一時保存
        if (base64 && base64.startsWith('data:image')) {
            const base64Data = base64.replace(/^data:image\/jpeg;base64,/, '');
            fs.writeFileSync(tempImagePath, Buffer.from(base64Data, 'base64'));
        } else {
            return { success: false, stderr: '画像データがありません' };
        }

        // FFmpegコマンド
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
        ffmpeg.stdout.on('data', data => stdout += data.toString());
        ffmpeg.stderr.on('data', data => stderr += data.toString());

        return new Promise((resolve, reject) => {
            ffmpeg.on('close', (code) => {
                if (code === 0) resolve({ success: true, stdout });
                else reject({ success: false, stderr });
            });
        });
    } catch (error) {
        return { success: false, error: error.message };
    }
});

// 設定取得
ipcMain.handle('get-config', async () => {
    const cfg = loadConfig();
    if (!cfg || !cfg.ffmpegPath || !fs.existsSync(cfg.ffmpegPath)) {
        return {
            ffmpegPath: HARDCODED_FFMPEG_PATH,
            fallback: true
        };
    }
    return cfg;
});

// 画像選択
ipcMain.handle('select-image', async () => {
    const result = await dialog.showOpenDialog({
        properties: ['openFile'],
        filters: [{ name: '画像', extensions: ['jpg', 'jpeg', 'png'] }]
    });
    if (result.canceled || !result.filePaths[0]) return null;
    return result.filePaths[0];
});

// メインウィンドウ生成
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

    // IPCハンドラをwindowに登録
    ipcMain.handle('generate-mp4-with-suno-cover', async (event, { url }) => {
        // 既存のgenerate-mp4-with-suno-coverの処理を再実装
    });

    ipcMain.handle('generate-mp4-with-base64', async (event, { url, base64 }) => {
        // 既存のgenerate-mp4-with-base64の処理を再実装
    });
}