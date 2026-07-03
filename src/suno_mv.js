const fs = require("fs");
const path = require("path");
const https = require("https");
const { exec } = require("child_process");

function download(url, dest) {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(dest);
        https.get(url, (response) => {
            if (response.statusCode !== 200) {
                return reject(`Failed to get '${url}' (${response.statusCode})`);
            }
            response.pipe(file);
            file.on("finish", () => file.close(resolve));
        }).on("error", (err) => {
            fs.unlink(dest, () => reject(err));
        });
        file.on("error", (err) => {
            fs.unlink(dest, () => reject(err));
        });
    });
}

function extractSunoId(url) {
    const m = url.match(/song\/([a-f0-9-]+)/);
    return m ? m[1] : null;
}

function sanitizeSunoId(songId) {
    return songId.replace(/[^a-zA-Z0-9-_]/g, "");
}

function buildAssetUrls(songId) {
    const safeId = sanitizeSunoId(songId);
    return {
        safeId,
        mp3Url: `https://cdn1.suno.ai/${safeId}.mp3`,
        defaultImageUrl: `https://cdn2.suno.ai/image_large_${safeId}.jpeg`,
    };
}

function buildOutputPaths(outputPath, safeId) {
    return {
        mp3Path: path.join(outputPath, `${safeId}.mp3`),
        imgPath: path.join(outputPath, `${safeId}.jpeg`),
        mp4Path: path.join(outputPath, `${safeId}.mp4`),
    };
}

function buildFfmpegCommand({ mp3Path, imgPath, mp4Path, resolution, visualizer }) {
    return [
        'ffmpeg', '-y',
        '-i', `"${mp3Path}"`,
        '-loop', '1',
        '-i', `"${imgPath}"`,
        '-filter_complex',
        `"[0:a]showspectrum=s=${resolution}:mode=${visualizer}[spec];[1:v][spec]overlay=format=auto"`,
        '-shortest',
        '-c:v', 'libx264',
        '-c:a', 'aac',
        `"${mp4Path}"`
    ].join(' ');
}

async function main(argv = process.argv) {
    const [
        ,,
        inputUrl,
        outputPath = "./output",
        resolution = "1280x720",
        visualizer = "spectrum",
        imagePathOverride
    ] = argv;

    const songId = inputUrl ? extractSunoId(inputUrl) : null;
    if (!inputUrl || !songId) {
        console.error("Usage: node suno_mv.js <Suno Song URL>");
        process.exit(1);
    }

    if (!fs.existsSync(outputPath)) fs.mkdirSync(outputPath, { recursive: true });
    const { safeId, mp3Url, defaultImageUrl } = buildAssetUrls(songId);
    const { mp3Path, imgPath, mp4Path } = buildOutputPaths(outputPath, safeId);

    try {
        console.log("Downloading audio...");
        await download(mp3Url, mp3Path);
    } catch (e) {
        console.error("Failed to download MP3:", e);
        process.exit(2);
    }

    if (!imagePathOverride) {
        try {
            console.log("Downloading cover image...");
            await download(defaultImageUrl, imgPath);
        } catch (e) {
            console.error("Failed to download image:", e);
            process.exit(3);
        }
    } else {
        try {
            fs.copyFileSync(imagePathOverride, imgPath);
        } catch (e) {
            console.error("Failed to copy image override:", e);
            process.exit(4);
        }
    }

    const ffmpegCmd = buildFfmpegCommand({ mp3Path, imgPath, mp4Path, resolution, visualizer });

    console.log("Generating video with FFmpeg...");
    exec(ffmpegCmd, (err, stdout, stderr) => {
        if (err) {
            console.error("FFmpeg error:", stderr);
            return;
        }
        console.log(`✅ Done! Output saved to: ${mp4Path}`);
    });
}

if (require.main === module) {
    main();
}

module.exports = {
    download,
    extractSunoId,
    sanitizeSunoId,
    buildAssetUrls,
    buildOutputPaths,
    buildFfmpegCommand,
    main,
};
