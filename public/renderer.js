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

        const isSunoCover = previewImg.getAttribute("data-cover-id") !== "custom";
        const base64 = previewImg.getAttribute("data-base64");
        if (!isSunoCover && (!base64 || !base64.startsWith("data:image"))) {
            alert("画像をドロップするか、プレビュー画像を選択してください");
            return;
        }

        logArea.textContent = "動画生成中…\n";
        try {
            const result = await invoke("generate_mp4", {
                request: {
                    url,
                    base64: isSunoCover ? null : base64,
                    resolution: "1280x720",
                    visualizer: "combined"
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
