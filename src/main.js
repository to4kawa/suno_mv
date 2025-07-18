const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs');
const { spawn } = require('child_process');

const HARDCODED_FFMPEG_PATH = 'C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe';

// Sunoカバー画像 + MP3自動ダウンロード
ipcMain.handle('generate-mp4-with-suno-cover', async (event, { url }) => {
    try {
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) return { success: false, stderr: 'Suno曲のURLが無効です' };
        const id = m[1];
        const appPath = app.getPath('userData');
        if (!fs.existsSync(appPath)) fs.mkdirSync(appPath, { recursive: true });

        // カバー画像
        const tempImagePath = path.join(appPath, 'temp_cover.jpg');
        const imgRes = await fetch(`https://cdn2.suno.ai/image_large_${id}.jpeg`);
        const imgBuf = Buffer.from(await imgRes.arrayBuffer());
        fs.writeFileSync(tempImagePath, imgBuf);

        // MP3
        const tempMp3Path = path.join(appPath, `${id}.mp3`);
        if (!fs.existsSync(tempMp3Path)) {
            const mp3Res = await fetch(`https://cdn1.suno.ai/${id}.mp3`);
            const mp3Buf = Buffer.from(await mp3Res.arrayBuffer());
            fs.writeFileSync(tempMp3Path, mp3Buf);
        }
        const outputVideoPath = path.join(appPath, `${id}.mp4`);
        const ffmpegArgs = [
            '-loop', '1', '-i', tempImagePath,
            '-i', tempMp3Path,
            '-c:v', 'libx264', '-preset', 'fast',
            '-pix_fmt', 'yuv420p',
            '-c:a', 'aac', '-shortest',
            '-movflags', '+faststart',
            outputVideoPath
        ];

        const ffmpeg = spawn(HARDCODED_FFMPEG_PATH, ffmpegArgs);
        let stdout = '', stderr = '';
        ffmpeg.stdout.on('data', data => stdout += data.toString());
        ffmpeg.stderr.on('data', data => stderr += data.toString());

        return await new Promise((resolve) => {
            ffmpeg.on('close', (code) => {
                if (code === 0) resolve({ success: true, stdout });
                else resolve({ success: false, stderr });
            });
        });
    } catch (error) {
        return { success: false, error: error.message };
    }
});

// base64画像
ipcMain.handle('generate-mp4-with-base64', async (event, { url, base64 }) => {
    try {
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) return { success: false, stderr: 'Suno曲のURLが無効です' };
        const id = m[1];
        const appPath = app.getPath('userData');
        if (!fs.existsSync(appPath)) fs.mkdirSync(appPath, { recursive: true });

        // 画像保存
        const tempImagePath = path.join(appPath, 'temp_cover.jpg');
        if (base64 && base64.startsWith('data:image')) {
            const base64Data = base64.split(',')[1];
            fs.writeFileSync(tempImagePath, Buffer.from(base64Data, 'base64'));
        } else {
            return { success: false, stderr: '画像データがありません' };
        }

        // MP3
        const tempMp3Path = path.join(appPath, `${id}.mp3`);
        if (!fs.existsSync(tempMp3Path)) {
            const mp3Res = await fetch(`https://cdn1.suno.ai/${id}.mp3`);
            const mp3Buf = Buffer.from(await mp3Res.arrayBuffer());
            fs.writeFileSync(tempMp3Path, mp3Buf);
        }

        const outputVideoPath = path.join(appPath, 'output.mp4');
        const ffmpegArgs = [
            '-loop', '1', '-i', tempImagePath,
            '-i', tempMp3Path,
            '-c:v', 'libx264', '-preset', 'fast',
            '-pix_fmt', 'yuv420p',
            '-c:a', 'aac', '-shortest',
            '-movflags', '+faststart',
            outputVideoPath
        ];

        const ffmpeg = spawn(HARDCODED_FFMPEG_PATH, ffmpegArgs);
        let stdout = '', stderr = '';
        ffmpeg.stdout.on('data', data => stdout += data.toString());
        ffmpeg.stderr.on('data', data => stderr += data.toString());

        return await new Promise((resolve) => {
            ffmpeg.on('close', (code) => {
                if (code === 0) resolve({ success: true, stdout });
                else resolve({ success: false, stderr });
            });
        });
    } catch (error) {
        return { success: false, error: error.message };
    }
});

// 画像選択や設定関連は省略（上のままでOK）

function createWindow() {
    const win = new BrowserWindow({
        width: 1080, height: 800,
        webPreferences: {
            preload: path.join(__dirname, 'preload.js'),
            contextIsolation: true,
            sandbox: true
        }
    });
    win.loadFile(path.join(__dirname, '../public/index.html'));
}

app.whenReady().then(createWindow);
app.on("window-all-closed", () => { if (process.platform !== "darwin") app.quit(); });
app.on("activate", () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });
