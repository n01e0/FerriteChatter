# FerriteChatter

ChatGPTとターミナル上で会話できるやつ。

名前はChatGPTに考えて貰った。

## Usage
OPEN AI API key を環境変数 `OPENAI_API_KEY` に設定。

あとは実行するだけ

### Chat mode
おしゃべりモード（素のChat-GPT）

```
$ cargo run --bin fchat
```

### Ask mode 
技術に関する質問に特化したモードです

```
$ cargo run --bin fask "質問文"
```

### Translate mode
日英翻訳に特化したモードです

```
$ cargo run --bin ftrans "翻訳したい文章（日本語）"
```

