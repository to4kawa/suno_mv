const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const https = require('https');
const { exec } = require('child_process');

// 設定ファイルの読み込み（ffmpegパス）
let ffmpegPath = 'ffmpeg'; // デフォルト
try {
  const setting = JSON.parse(fs.readFileSync(path.join(__dirname, '../setting.json'), 'utf-8'));
  if (setting.ffmpegPath) ffmpegPath = `"${setting.ffmpegPath}"`;
} catch(e) {
  // 設定ファイルなし→デフォルトでffmpeg
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode !== 200) return reject(`HTTP ${response.statusCode}`);
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
    }).on("error", reject);
    file.on("error", (err) => {
      fs.unlink(dest, () => reject(err));
    });
  });
}

// --- generate-mp4 Handler ---
ipcMain.handle('generate-mp4', async (event, params) => {
  try {
    const { url, imagePath } = params;
    const songId = (url.match(/song\/([a-f0-9-]+)/) || [])[1];
    if (!songId) throw new Error("Invalid Suno URL!");

    const outputDir = path.join(__dirname, '..', 'output');
    if (!fs.existsSync(outputDir)) fs.mkdirSync(outputDir);

    const mp3Url = `https://cdn1.suno.ai/${songId}.mp3`;
    const imgUrl = `https://cdn2.suno.ai/image_large_${songId}.jpeg`;
    const mp3Path = path.join(outputDir, `${songId}.mp3`);
    const imgPath = path.join(outputDir, `${songId}.jpg`);
    const mp4Path = path.join(outputDir, `${songId}.mp4`);

    // mp3ダウンロード
    if (!fs.existsSync(mp3Path)) await download(mp3Url, mp3Path);

    // カバー画像: ローカル指定か自動DL
    if (imagePath && fs.existsSync(imagePath)) {
      fs.copyFileSync(imagePath, imgPath);
    } else if (!fs.existsSync(imgPath)) {
      await download(imgUrl, imgPath);
    }

    // ffmpegコマンド作成（drawtextなしのシンプル版／必要なら追加可）
    const cmd = `${ffmpegPath} -y -i "${mp3Path}" -loop 1 -i "${imgPath}" -shortest -c:v libx264 -profile:v high -pix_fmt yuv420p -c:a aac "${mp4Path}"`;

    // 実行
    await new Promise((resolve, reject) => {
      exec(cmd, (err, stdout, stderr) => {
        if (err) reject(stderr);
        else resolve(stdout);
      });
    });

    return { success: true, mp4Path };

  } catch (err) {
    return { success: false, error: err.toString() };
  }
});
