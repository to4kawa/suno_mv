// main.js 修正版
const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs-extra');
const { spawn } = require('child_process');

const HARDCODED_FFMPEG_PATH = 'C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe';

// ✅ ハンドラ名を修正
ipcMain.handle('generate-mp4-with-suno-cover', async (event, { url }) => {
    const m = url.match(/song\/([a-f0-9-]+)/);
    if (!m) return { success: false, stderr: 'URLが無効です' };

    const id = m[1];
    const appPath = app.getPath('userData');
    const tempImagePath = path.join(appPath, 'temp_cover.jpg');
    const outputVideoPath = path.join(appPath, 'output.mp4');

    // 一時ディレクトリがなければ作成
    if (!fs.existsSync(appPath)) fs.mkdirpSync(appPath);

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
            if (code === 0) {
                resolve({ success: true, stdout });
            } else {
                reject({ success: false, stderr });
            }
        });
    });
});