window.onload = () => {
    const urlInput = document.getElementById("url-input");
    const previewBtn = document.getElementById("preview-btn");
    const generateBtn = document.getElementById("generate-btn");
    const dropArea = document.getElementById("drop-area");
    const previewImg = document.getElementById("preview-img");
    const logArea = document.getElementById("log");

    const getSunoId = (url) => {
        const m = url.match(/song\/([a-f0-9-]+)/);
        return m ? m[1] : null;
    };

    previewBtn.onclick = () => {
        const id = getSunoId(urlInput.value.trim());
        if (id) {
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
        dropArea.style.background = "#f0f0f0";
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
        if (!getSunoId(url)) {
            alert("Suno曲のURLを正しく入力してください");
            return;
        }

        const base64 = previewImg.getAttribute("data-cover-id") === "custom"
            ? previewImg.getAttribute("data-base64")
            : null;

        logArea.textContent = "動画生成中…\n";

        try {
            const { invoke } = window.__TAURI__.core;
            const result = await invoke("generate_mp4", {
                request: {
                    url,
                    base64,
                    resolution: "1280x720",
                    visualizer: "spectrum"
                }
            });

            if (result.success) {
                alert(`✅ 完了！出力先: ${result.output_path}`);
            } else {
                alert("動画生成中にエラーが発生しました\n\n" + (result.stderr || "詳細不明"));
            }
            logArea.textContent += `${result.stdout || ""}\n${result.stderr || ""}`;
        } catch (e) {
            alert("Tauri IPCエラー:\n\n" + e.message);
            logArea.textContent += "IPCエラー: " + e.message;
        }

        logArea.scrollTop = logArea.scrollHeight;
    };
};
