const invoke = async (command, args) => {
    const tauriInvoke = window.__TAURI__?.core?.invoke;
    if (!tauriInvoke) {
        throw new Error("Tauri IPC is not available. Start the app with `npm start` or `npm run dev`.");
    }
    return tauriInvoke(command, args);
};

window.onload = () => {
    const urlInput = document.getElementById("url-input");
    const previewBtn = document.getElementById("preview-btn");
    const generateBtn = document.getElementById("generate-btn");
    const dropArea = document.getElementById("drop-area");
    const previewImg = document.getElementById("preview-img");
    const logArea = document.getElementById("log");
    const saveFolderInput = document.getElementById("save-folder-input");
    const chooseFolderBtn = document.getElementById("choose-folder-btn");
    const openFolderBtn = document.getElementById("open-folder-btn");
    const ffmpegAutoDetectInput = document.getElementById("ffmpeg-auto-detect-input");
    const ffmpegPathInput = document.getElementById("ffmpeg-path-input");
    const chooseFfmpegBtn = document.getElementById("choose-ffmpeg-btn");
    const testFfmpegBtn = document.getElementById("test-ffmpeg-btn");
    const ffmpegStatus = document.getElementById("ffmpeg-status");
    const encoderPresetInput = document.getElementById("encoder-preset-input");
    const qualityInput = document.getElementById("quality-input");

    const defaultSettings = {
        save_folder: "",
        ffmpeg_auto_detect: true,
        ffmpeg_path: "",
        encoder_preset: "cpu_x264",
        quality: "standard"
    };

    const getSettingsFromUi = () => ({
        save_folder: saveFolderInput.value.trim(),
        ffmpeg_auto_detect: ffmpegAutoDetectInput.checked,
        ffmpeg_path: ffmpegPathInput.value.trim(),
        encoder_preset: encoderPresetInput.value,
        quality: qualityInput.value
    });

    const applySettingsToUi = (settings) => {
        const merged = { ...defaultSettings, ...(settings || {}) };
        saveFolderInput.value = merged.save_folder || "";
        ffmpegAutoDetectInput.checked = Boolean(merged.ffmpeg_auto_detect);
        ffmpegPathInput.value = merged.ffmpeg_path || "";
        encoderPresetInput.value = merged.encoder_preset || "cpu_x264";
        qualityInput.value = merged.quality || "standard";
        syncFfmpegControls();
    };

    const saveSettings = async () => {
        await invoke("save_settings", { settings: getSettingsFromUi() });
    };

    function syncFfmpegControls() {
        const autoDetect = ffmpegAutoDetectInput.checked;
        ffmpegPathInput.disabled = autoDetect;
        chooseFfmpegBtn.disabled = autoDetect;
    }

    const setFfmpegStatus = (result) => {
        if (result.success) {
            ffmpegStatus.textContent = `Status: OK\n${result.version || "FFmpeg found"}`;
        } else {
            ffmpegStatus.textContent = `Status: ${result.status || "Not found"}\n${result.details || "FFmpeg was not found."}`;
        }
    };

    const blobToDataUrl = (blob) => new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result);
        reader.onerror = () => reject(reader.error || new Error("画像を読み込めませんでした"));
        reader.readAsDataURL(blob);
    });

    const resolveCoverBase64 = async () => {
        const droppedBase64 = previewImg.getAttribute("data-base64");
        if (droppedBase64 && droppedBase64.startsWith("data:image")) {
            return droppedBase64;
        }

        const src = previewImg.getAttribute("src");
        if (!src) {
            return null;
        }
        if (src.startsWith("data:image")) {
            return src;
        }

        try {
            const response = await fetch(src, { mode: "cors" });
            if (!response.ok) {
                logArea.textContent += `\nPreview image fetch failed: ${response.status} ${response.statusText}`;
                return null;
            }
            const blob = await response.blob();
            if (!blob.type.startsWith("image/")) {
                logArea.textContent += `\nPreview image fetch did not return an image: ${blob.type || "unknown type"}`;
                return null;
            }
            return await blobToDataUrl(blob);
        } catch (e) {
            logArea.textContent += "\nPreview image fetch failed: " + e.message;
            return null;
        }
    };

    invoke("load_settings")
        .then(applySettingsToUi)
        .catch((e) => {
            applySettingsToUi(defaultSettings);
            ffmpegStatus.textContent = "Status: Settings load failed\n" + e.message;
        });

    [saveFolderInput, ffmpegPathInput, encoderPresetInput, qualityInput].forEach((element) => {
        element.addEventListener("change", () => {
            saveSettings().catch((e) => {
                logArea.textContent += "\nSettings save failed: " + e.message;
            });
        });
    });

    ffmpegAutoDetectInput.addEventListener("change", () => {
        syncFfmpegControls();
        saveSettings().catch((e) => {
            logArea.textContent += "\nSettings save failed: " + e.message;
        });
    });

    chooseFolderBtn.onclick = async () => {
        const chosen = prompt("Save folder path", saveFolderInput.value);
        if (chosen !== null) {
            saveFolderInput.value = chosen.trim();
            await saveSettings();
        }
    };

    chooseFfmpegBtn.onclick = async () => {
        const chosen = prompt("ffmpeg.exe path", ffmpegPathInput.value);
        if (chosen !== null) {
            ffmpegPathInput.value = chosen.trim();
            await saveSettings();
        }
    };

    openFolderBtn.onclick = async () => {
        try {
            await saveSettings();
            const opened = await invoke("open_output_folder", {
                saveFolder: saveFolderInput.value.trim() || null
            });
            logArea.textContent += `\nOpened folder: ${opened}`;
        } catch (e) {
            alert("フォルダを開けませんでした\n\n" + e.message);
        }
    };

    testFfmpegBtn.onclick = async () => {
        ffmpegStatus.textContent = "Status: Testing...";
        try {
            await saveSettings();
            const result = await invoke("test_ffmpeg", { settings: getSettingsFromUi() });
            setFfmpegStatus(result);
        } catch (e) {
            ffmpegStatus.textContent = "Status: Test failed\n" + e.message;
        }
    };

    previewBtn.onclick = () => {
        const url = urlInput.value.trim();
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (m) {
            const id = m[1];
            previewImg.src = `https://cdn2.suno.ai/image_large_${id}.jpeg`;
            previewImg.setAttribute("data-cover-id", id);
            previewImg.removeAttribute("data-base64");
        } else {
            previewImg.src = "";
            alert("Suno曲のURLを正しく入力してください");
        }
    };

    dropArea.ondragover = (e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "copy";
        dropArea.style.background = "#f5f5f5";
    };

    dropArea.ondragleave = () => {
        dropArea.style.background = "";
    };

    dropArea.ondrop = (e) => {
        e.preventDefault();
        dropArea.style.background = "";
        const file = e.dataTransfer.files[0];
        if (file && file.type.startsWith("image/")) {
            const reader = new FileReader();
            reader.onload = (event) => {
                const base64 = event.target.result;
                previewImg.src = base64;
                previewImg.setAttribute("data-cover-id", "custom");
                previewImg.setAttribute("data-base64", base64);
                logArea.textContent += "\nCover image loaded from local file.";
            };
            reader.onerror = () => {
                alert("画像ファイルを読み込めませんでした");
            };
            reader.readAsDataURL(file);
        } else {
            alert("画像ファイルをドロップしてください");
        }
    };

    generateBtn.onclick = async () => {
        const url = urlInput.value.trim();
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) {
            alert("Suno曲のURLを正しく入力してください");
            return;
        }

        logArea.textContent = "動画生成中…\n";
        try {
            await saveSettings();
            const base64 = await resolveCoverBase64();
            if (base64) {
                logArea.textContent += "Using frontend cover image data.\n";
            } else {
                logArea.textContent += "No frontend cover image data; Rust will try the cdn2 fallback.\n";
            }
            const result = await invoke("generate_mp4", {
                request: {
                    url,
                    base64,
                    resolution: "1280x720",
                    visualizer: "combined",
                    outputDir: saveFolderInput.value.trim() || null,
                    ffmpegAutoDetect: ffmpegAutoDetectInput.checked,
                    ffmpegPath: ffmpegPathInput.value.trim(),
                    encoderPreset: encoderPresetInput.value,
                    quality: qualityInput.value
                }
            });

            if (result.success) {
                alert(`✅ 完了！出力先: ${result.outputPath || "output"}`);
                logArea.textContent += result.stdout || result.stderr || "完了しました";
            } else {
                alert("動画生成中にエラーが発生しました\n\n" + (result.stderr || "詳細不明"));
                logArea.textContent += (result.stderr || "") + "\n" + (result.stdout || "");
            }
        } catch (e) {
            alert("IPC通信エラー:\n\n" + e.message);
            logArea.textContent += "IPCエラー: " + e.message;
        }

        logArea.scrollTop = logArea.scrollHeight;
    };
};
