window.onload = () => {
    const urlInput = document.getElementById("url-input");
    const previewBtn = document.getElementById("preview-btn");
    const generateBtn = document.getElementById("generate-btn");
    const dropArea = document.getElementById("drop-area");
    const previewImg = document.getElementById("preview-img");
    const logArea = document.getElementById("log");
    const settingsBtn = document.getElementById("settings-btn");
    const outputDirDisplay = document.getElementById("output-dir");

    window.electronAPI.getOutputDir().then(dir => {
        if (dir) outputDirDisplay.textContent = `Output: ${dir}`;
    });

    settingsBtn.onclick = async () => {
        const res = await window.electronAPI.selectOutputDir();
        if (res && res.success) {
            outputDirDisplay.textContent = `Output: ${res.outputDir}`;
        }
    };

    // プレビュー
    previewBtn.onclick = () => {
        const url = urlInput.value.trim();
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (m) {
            const id = m[1];
            previewImg.src = `https://cdn2.suno.ai/image_large_${id}.jpeg`;
            previewImg.setAttribute("data-cover-id", id);
            previewImg.removeAttribute("data-file");
            previewImg.removeAttribute("data-base64");
        } else {
            previewImg.src = "";
            alert("Suno曲のURLを正しく入力してください");
        }
    };

    // 画像ドラッグ＆ドロップ
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
                previewImg.removeAttribute("data-file");
            };
            reader.readAsDataURL(file);
        } else {
            alert("画像ファイルをドロップしてください");
        }
    };

    window.addEventListener('paste', (e) => {
        const items = e.clipboardData && e.clipboardData.items;
        if (!items) return;
        for (const item of items) {
            if (item.type.startsWith('image/')) {
                const file = item.getAsFile();
                if (file) {
                    const reader = new FileReader();
                    reader.onload = (event) => {
                        const base64 = event.target.result;
                        previewImg.src = base64;
                        previewImg.setAttribute('data-cover-id', 'custom');
                        previewImg.setAttribute('data-base64', base64);
                        previewImg.removeAttribute('data-file');
                    };
                    reader.readAsDataURL(file);
                    e.preventDefault();
                    break;
                }
            }
        }
    });

    // 動画生成
    generateBtn.onclick = async () => {
        const url = urlInput.value.trim();
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) {
            alert("Suno曲のURLを正しく入力してください");
            return;
        }

        const isSunoCover = previewImg.getAttribute("data-cover-id") !== "custom";
        const base64 = previewImg.getAttribute("data-base64");

        if (!isSunoCover && (!base64 || !base64.startsWith('data:image'))) {
            alert("画像をドロップするか、プレビュー画像を選択してください");
            return;
        }

        logArea.textContent = "動画生成中…\n";

        try {
            let result;
            if (isSunoCover) {
                // ✅ ハンドラ名を統一
                result = await window.electronAPI.generateMP4WithSunoCover({ url });
            } else {
                result = await window.electronAPI.generateMP4WithBase64({ url, base64 });
            }

            if (result.success) {
                alert("✅ 完了！outputフォルダを確認してください");
                logArea.textContent += result.stdout;
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
