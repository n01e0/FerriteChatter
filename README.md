# FerriteChatter

ChatGPTとターミナル上で会話できるやつ。

名前はChatGPTに考えて貰った。

## Usage
API keyは`OPENAI_API_KEY`に設定。

あとは実行するだけ

```bash
# Chat形式
# "exit"で終了、"reset"で会話のリセット、"v"でエディターを使用した入力ができる。
$ fchat

# 単発の質問
$ fask

# 日英・英日翻訳
$ ftrans

# 基本的に共通のオプション
$ fchat -h
Usage: fchat [OPTIONS]

Options:
  -g, --general <GENERAL>  Open Prompt(General Prompt)
  -k, --key <KEY>          OenAI API Key
  -m, --model <MODEL>      default is "gpt-4-32k" [default: gpt-4] [possible values: gpt-4, gpt-4-0314, gpt-4-0613, gpt-4-32k, gpt-4-32k-0613, gpt-3.5-turbo, gpt-3.5-turbo-16k, gpt-3.5-turbo-0301, gpt-3.5-turbo-0613, gpt-3.5-turbo-16k-0613]
  -h, --help               Print help
  -V, --version            Print version
```

## installation

```bash
cargo install FerriteChatter
```
