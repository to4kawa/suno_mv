const invoke = async (command, args) => {
    const tauriInvoke = window.__TAURI__?.core?.invoke;
    if (!tauriInvoke) {
        throw new Error("Tauri IPC is not available. Start the app with `npm start` or `npm run dev`.");
    }
    return tauriInvoke(command, args);
};

const extractSunoId = (url) => {
    const match = url.match(/suno\.com\/(?:song|s)\/([a-f0-9-]+)/i);
    return match ? match[1] : null;
};

const maskSecret = (value) => {
    const trimmed = (value || "").trim();
    if (!trimmed) return "";
    const parts = trimmed.split(/\s+/, 2);
    if (parts.length === 2) {
        return `${parts[0]} ${maskToken(parts[1])}`;
    }
    return maskToken(trimmed);
};

const maskToken = (value) => {
    if (value.length <= 6) return "***";
    return `${value.slice(0, 3)}...${value.slice(-3)}`;
};

const hasSunoApiSettings = (settings) => Boolean(
    settings.suno_authorization?.trim() &&
    settings.suno_browser_token?.trim() &&
    settings.suno_device_id?.trim()
);

const selectAudioSource = (metadata, fallback) => metadata?.audioUrl || fallback;

const selectCoverSource = (frontendData, metadata, fallback) => {
    if (frontendData) return frontendData;
    return metadata?.imageLargeUrl || metadata?.imageUrl || fallback;
};

window.onload = () => {
    const urlInput = document.getElementById("url-input");
    const previewBtn = document.getElementById("preview-btn");
    const generateBtn = document.getElementById("generate-btn");
    const dropArea = document.getElementById("drop-area");
    const previewImg = document.getElementById("preview-img");
    const logArea = document.getElementById("log");
    const statusLine = document.getElementById("status-line");
    const outputResult = document.getElementById("output-result");
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
    const resolutionInput = document.getElementById("resolution-input");
    const visualizerInput = document.getElementById("visualizer-input");
    const sunoAuthorizationInput = document.getElementById("suno-authorization-input");
    const sunoBrowserTokenInput = document.getElementById("suno-browser-token-input");
    const sunoDeviceIdInput = document.getElementById("suno-device-id-input");
    const sunoMaxPagesInput = document.getElementById("suno-max-pages-input");
    const testMetadataBtn = document.getElementById("test-metadata-btn");
    const sunoApiStatus = document.getElementById("suno-api-status");
    const metadataOutput = document.getElementById("metadata-output");
    const lyricsOutput = document.getElementById("lyrics-output");
    const promptOutput = document.getElementById("prompt-output");

    let currentMetadata = null;
    let currentSongId = null;

    const defaultSettings = {
        save_folder: "",
        ffmpeg_auto_detect: true,
        ffmpeg_path: "",
        encoder_preset: "cpu_x264",
        quality: "standard",
        suno_authorization: "",
        suno_browser_token: "",
        suno_device_id: "",
        suno_max_pages: 100
    };

    const setStatus = (status) => {
        statusLine.textContent = status;
    };

    const appendLog = (message) => {
        logArea.textContent += `${message}\n`;
        logArea.scrollTop = logArea.scrollHeight;
    };

    const resetMetadataUi = () => {
        currentMetadata = null;
        metadataOutput.textContent = "No metadata loaded.";
        lyricsOutput.textContent = "No lyrics metadata loaded.";
        promptOutput.textContent = "No prompt metadata loaded.";
    };

    const renderMetadata = (metadata) => {
        if (!metadata) {
            resetMetadataUi();
            return;
        }

        metadataOutput.textContent = [
            ["title", metadata.title],
            ["id", metadata.id],
            ["display_name", metadata.displayName],
            ["created_at", metadata.createdAt],
            ["audio_url", metadata.audioUrl],
            ["image_url", metadata.imageUrl],
            ["image_large_url", metadata.imageLargeUrl],
            ["video_url", metadata.videoUrl]
        ]
            .filter(([, value]) => value)
            .map(([label, value]) => `${label}: ${value}`)
            .join("\n") || "No metadata loaded.";
        lyricsOutput.textContent = metadata.lyrics || "No lyrics metadata loaded.";
        promptOutput.textContent = metadata.prompt || "No prompt metadata loaded.";
    };

    const getSettingsFromUi = () => ({
        save_folder: saveFolderInput.value.trim(),
        ffmpeg_auto_detect: ffmpegAutoDetectInput.checked,
        ffmpeg_path: ffmpegPathInput.value.trim(),
        encoder_preset: encoderPresetInput.value,
        quality: qualityInput.value,
        suno_authorization: sunoAuthorizationInput.value.trim(),
        suno_browser_token: sunoBrowserTokenInput.value.trim(),
        suno_device_id: sunoDeviceIdInput.value.trim(),
        suno_max_pages: Math.max(1, Number.parseInt(sunoMaxPagesInput.value, 10) || 100)
    });

    const applySettingsToUi = (settings) => {
        const merged = { ...defaultSettings, ...(settings || {}) };
        saveFolderInput.value = merged.save_folder || "";
        ffmpegAutoDetectInput.checked = Boolean(merged.ffmpeg_auto_detect);
        ffmpegPathInput.value = merged.ffmpeg_path || "";
        encoderPresetInput.value = merged.encoder_preset || "cpu_x264";
        qualityInput.value = merged.quality || "standard";
        sunoAuthorizationInput.value = merged.suno_authorization || "";
        sunoBrowserTokenInput.value = merged.suno_browser_token || "";
        sunoDeviceIdInput.value = merged.suno_device_id || "";
        sunoMaxPagesInput.value = merged.suno_max_pages || 100;
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
                appendLog(`preview cover fetch failed: ${response.status} ${response.statusText}`);
                return null;
            }
            const blob = await response.blob();
            if (!blob.type.startsWith("image/")) {
                appendLog(`preview cover fetch did not return an image: ${blob.type || "unknown type"}`);
                return null;
            }
            return await blobToDataUrl(blob);
        } catch (e) {
            appendLog("preview cover fetch failed: " + e.message);
            return null;
        }
    };

    const fetchMetadata = async (url) => {
        const settings = getSettingsFromUi();
        if (!hasSunoApiSettings(settings)) {
            appendLog("metadata fetch skipped: Suno API settings are not configured");
            setStatus("Preview OK");
            sunoApiStatus.textContent = "Suno API settings are not configured.";
            return null;
        }

        await saveSettings();
        appendLog("metadata fetch start");
        appendLog(`authorization ${maskSecret(settings.suno_authorization)}`);
        appendLog(`browser-token ${maskSecret(settings.suno_browser_token)}`);
        appendLog(`device-id ${maskSecret(settings.suno_device_id)}`);
        const result = await invoke("fetch_suno_metadata", { url });
        (result.logs || []).forEach(appendLog);
        sunoApiStatus.textContent = result.status || "Metadata fetch finished.";
        if (result.success && result.metadata) {
            currentMetadata = result.metadata;
            renderMetadata(currentMetadata);
            setStatus("Metadata found");
            return currentMetadata;
        }
        setStatus("Metadata unavailable, using fallback");
        return null;
    };

    invoke("load_settings")
        .then(applySettingsToUi)
        .catch((e) => {
            applySettingsToUi(defaultSettings);
            ffmpegStatus.textContent = "Status: Settings load failed\n" + e.message;
        });

    [
        saveFolderInput,
        ffmpegPathInput,
        encoderPresetInput,
        qualityInput,
        sunoAuthorizationInput,
        sunoBrowserTokenInput,
        sunoDeviceIdInput,
        sunoMaxPagesInput
    ].forEach((element) => {
        element.addEventListener("change", () => {
            saveSettings().catch((e) => appendLog("settings save failed: " + e.message));
        });
    });

    ffmpegAutoDetectInput.addEventListener("change", () => {
        syncFfmpegControls();
        saveSettings().catch((e) => appendLog("settings save failed: " + e.message));
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
            appendLog(`opened folder: ${opened}`);
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

    testMetadataBtn.onclick = async () => {
        const url = urlInput.value.trim();
        if (!extractSunoId(url)) {
            sunoApiStatus.textContent = "Enter a Suno URL first.";
            return;
        }
        try {
            const metadata = await fetchMetadata(url);
            sunoApiStatus.textContent = metadata ? "Metadata found" : "Metadata unavailable, using fallback";
        } catch (e) {
            sunoApiStatus.textContent = "Metadata fetch failed: " + e.message;
            appendLog("metadata fetch failed: " + e.message);
        }
    };

    previewBtn.onclick = async () => {
        const url = urlInput.value.trim();
        const id = extractSunoId(url);
        if (!id) {
            previewImg.src = "";
            setStatus("Failed");
            alert("Suno曲のURLを正しく入力してください");
            return;
        }

        currentSongId = id;
        resetMetadataUi();
        outputResult.textContent = "";
        previewImg.src = `https://cdn2.suno.ai/${id}.jpeg`;
        previewImg.setAttribute("data-cover-id", id);
        previewImg.removeAttribute("data-base64");
        appendLog(`id extraction: ${id}`);
        appendLog(`preview cover source: cdn2 fallback https://cdn2.suno.ai/${id}.jpeg`);
        setStatus("Preview OK");

        try {
            const metadata = await fetchMetadata(url);
            const metadataCover = metadata?.imageLargeUrl || metadata?.imageUrl;
            if (metadataCover && previewImg.getAttribute("data-cover-id") !== "custom") {
                previewImg.src = metadataCover;
                appendLog(`preview cover source: metadata ${metadataCover}`);
            }
        } catch (e) {
            appendLog("metadata fetch failed: " + e.message);
            setStatus("Metadata unavailable, using fallback");
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
                appendLog("preview cover source: dropped frontend data URL");
                setStatus("Preview OK");
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
        const id = extractSunoId(url);
        if (!id) {
            alert("Suno曲のURLを正しく入力してください");
            return;
        }

        setStatus("Rendering");
        outputResult.textContent = "";
        try {
            await saveSettings();
            const fallbackAudio = `https://cdn1.suno.ai/${id}.mp3`;
            const fallbackCover = `https://cdn2.suno.ai/${id}.jpeg`;
            const metadataForRequest = currentSongId === id ? currentMetadata : null;
            const base64 = await resolveCoverBase64();
            const selectedAudio = selectAudioSource(metadataForRequest, fallbackAudio);
            const selectedCover = selectCoverSource(base64, metadataForRequest, fallbackCover);
            appendLog(`selected audio source: ${selectedAudio}`);
            appendLog(`selected cover source: ${base64 ? "frontend data URL" : selectedCover}`);

            const result = await invoke("generate_mp4", {
                request: {
                    url,
                    base64,
                    resolution: resolutionInput.value,
                    visualizer: visualizerInput.value,
                    outputDir: saveFolderInput.value.trim() || null,
                    ffmpegAutoDetect: ffmpegAutoDetectInput.checked,
                    ffmpegPath: ffmpegPathInput.value.trim(),
                    encoderPreset: encoderPresetInput.value,
                    quality: qualityInput.value,
                    audioUrl: metadataForRequest?.audioUrl || null,
                    metadataImageUrl: metadataForRequest?.imageLargeUrl || metadataForRequest?.imageUrl || null,
                    outputFilename: metadataForRequest?.title || null
                }
            });

            if (result.success) {
                setStatus("Done");
                outputResult.textContent = result.outputPath || "output";
                appendLog(`output path: ${result.outputPath || "output"}`);
                if (result.stderr) appendLog(`FFmpeg stderr:\n${result.stderr}`);
                if (result.stdout) appendLog(`FFmpeg stdout:\n${result.stdout}`);
            } else {
                setStatus("Failed");
                appendLog(result.stderr || "render failed without details");
                alert("動画生成中にエラーが発生しました\n\n" + (result.stderr || "詳細不明"));
            }
        } catch (e) {
            setStatus("Failed");
            appendLog("IPC error: " + e.message);
            alert("IPC通信エラー:\n\n" + e.message);
        }
    };
};
