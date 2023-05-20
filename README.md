# FerriteChatter

ChatGPTとターミナル上で会話できるやつ。

名前はChatGPTに考えて貰った。

## Usage
OPEN AI API key を環境変数 `OPENAI_API_KEY` に設定。

あとは実行するだけ

### Chat mode
対話モード

```
$ cargo run --bin fchat
```

### Ask mode 
一問一答モード

```
$ cargo run --bin fask "質問文"
```

### Translate mode
日英翻訳に特化したモード（対話モードで実行します）

```
$ cargo run --bin ftrans
```

