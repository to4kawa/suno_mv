
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
        });

        file.on("finish", () => file.close(resolve));
        file.on("error", (err) => {
            fs.unlink(dest, () => reject(err));
        });
    });
}

async function main() {
    const [,, inputUrl, outputPath = "./output", resolution = "1280x720", visualizer = "spectrum", imagePathOverride] = process.argv;

    if (!inputUrl || !inputUrl.includes("suno.com/song/")) {
        console.error("Usage: node suno_mv.js <Suno Song URL>");
        process.exit(1);
    }

    const songId = inputUrl.split("/song/")[1];
    if (!fs.existsSync(outputPath)) fs.mkdirSync(outputPath, { recursive: true });

    const mp3Url = `https://cdn.suno.ai/songs/${songId}.mp3`;
    const defaultImageUrl = `https://cdn.suno.ai/covers/${songId}.jpeg`;
    const mp3Path = path.join(outputPath, `${songId}.mp3`);
    const imgPath = path.join(outputPath, `${songId}.jpeg`);
    const mp4Path = path.join(outputPath, `${songId}.mp4`);

    console.log("Downloading audio...");
    await download(mp3Url, mp3Path);

    if (!imagePathOverride) {
        console.log("Downloading cover image...");
        await download(defaultImageUrl, imgPath);
    } else {
        fs.copyFileSync(imagePathOverride, imgPath);
    }

    console.log("Generating video with FFmpeg...");
    const ffmpegCmd = `ffmpeg -y -i "${mp3Path}" -loop 1 -i "${imgPath}" -filter_complex "[0:a]showspectrum=s=${resolution}:mode=${visualizer}[spec];[1:v][spec]overlay=format=auto" -shortest -c:v libx264 -c:a aac "${mp4Path}"`;

    exec(ffmpegCmd, (err, stdout, stderr) => {
        if (err) {
            console.error("FFmpeg error:", stderr);
            return;
        }
        console.log(`✅ Done! Output saved to: ${mp4Path}`);
    });
}

main();
