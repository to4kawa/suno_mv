window.onload = () => {
    const urlInput = document.getElementById("url-input");
    const previewBtn = document.getElementById("preview-btn");
    const generateBtn = document.getElementById("generate-btn");
    const dropArea = document.getElementById("drop-area");
    const previewImg = document.getElementById("preview-img");
    const logArea = document.getElementById("log");
    const settingsBtn = document.getElementById("settings-btn");

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
                previewImg.src = base64; // 🔁 base64でプレビュー
                previewImg.setAttribute("data-cover-id", "custom");
                previewImg.setAttribute("data-base64", base64);
                previewImg.removeAttribute("data-file");
            };
            reader.readAsDataURL(file);
        } else {
            alert("画像ファイルをドロップしてください");
        }
    };

    // 動画生成
    // ...（略）window.onload = () => { ... のなか

    generateBtn.onclick = async () => {
        const url = urlInput.value.trim();
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) {
            alert("Suno曲のURLを正しく入力してください");
            return;
        }

        // Suno画像プレビューか、カスタム画像（data-base64）がどちらか必須
        const isSunoCover = previewImg.getAttribute("data-cover-id") && previewImg.getAttribute("data-cover-id") !== "custom";
        const base64 = previewImg.getAttribute("data-base64");

        if (!isSunoCover && (!base64 || !base64.startsWith('data:image'))) {
            alert("画像をドロップするか、プレビュー画像を表示してください");
            return;
        }

        logArea.textContent = "動画生成中…\n";

        try {
            let result;
            if (isSunoCover) {
                // サーバ側で自動的にカバー画像を取得（base64送信不要）
                result = await window.electronAPI.generateMP4WithSunoCover({ url });
            } else {
                // カスタム画像
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
    };
