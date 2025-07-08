window.onload = () => {
    const urlInput = document.getElementById("url-input");
    const previewBtn = document.getElementById("preview-btn");
    const generateBtn = document.getElementById("generate-btn");
    const dropArea = document.getElementById("drop-area");
    const previewImg = document.getElementById("preview-img");
    const logArea = document.getElementById("log");
    const settingsBtn = document.getElementById("settings-btn"); // 設定ボタンを追加

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
    dropArea.ondragover = (e) => {
        e.preventDefault();
        dropArea.style.background = "#def";
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
                previewImg.src = event.target.result; // base64でプレビュー
                previewImg.setAttribute("data-cover-id", "custom");
                previewImg.setAttribute("data-base64", event.target.result); // ファイルパスではなくbase64を保存
                previewImg.removeAttribute("data-file");
            };
            reader.readAsDataURL(file);
        } else {
            alert("画像ファイルをドロップしてください");
        }
    };

    // 設定画面を開く
    settingsBtn.onclick = () => {
        window.electronAPI.openSettingsWindow();
    };

    // 動画生成
    generateBtn.onclick = async () => {
        const url = urlInput.value.trim();
        const m = url.match(/song\/([a-f0-9-]+)/);
        if (!m) {
            alert("Suno曲のURLを正しく入力してください");
            return;
        }

        let base64Image = null;
        if (previewImg.getAttribute("data-cover-id") === "custom") {
            base64Image = previewImg.getAttribute("data-base64");
        }

        logArea.textContent = "動画生成中…\n";

        // main.jsにbase64データを渡す
        const result = await window.electronAPI.generateMP4WithBase64({
            url,
            base64: base64Image
        });

        if (result.success) {
            alert("✅ 完了！outputフォルダを確認してください");
            logArea.textContent += result.stdout;
        } else {
            alert("動画生成中にエラーが発生しました\n\n" + (result.stderr || "詳細不明"));
            logArea.textContent += (result.stderr || "") + "\n" + (result.stdout || "");
        }
    };
};