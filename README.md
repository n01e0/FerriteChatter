# FerriteChatter

ChatGPTとターミナル上で会話できるやつ。

名前はChatGPTに考えて貰った。

## Usage
API keyは`OPENAI_API_KEY`に設定

もしくは`$XDG_CONFIG_HOME/.ferriteconf.yaml`に記載。

あとは実行するだけ

```bash
# Chat形式
# "exit"で終了、"reset"で会話のリセット、"v"でエディターを使用した入力ができる。
$ fchat

# 単発の質問 パイプまたは引数からの入力
$ fask

# 日英・英日翻訳 パイプまたは引数からの入力
$ ftrans

# 基本的に共通のオプション
# fchatのみ、ファイルからコンテキストを渡せます。

$ fchat -h
Usage: fchat [OPTIONS]

Options:
  -g, --general <GENERAL>  Open Prompt(General Prompt)
  -k, --key <KEY>          OenAI API Key
  -m, --model <MODEL>      OpenAI Model [possible values: chatgpt-4o-latest, gpt-4, gpt-4o, gpt-4o-2024-05-13, gpt-4o-2024-08-06, gpt-4o-mini, gpt-4o-mini-2024-07-18, gpt-4-0314, gpt-4-0613, gpt-4-32k, gpt-4-32k-0613, gpt-4-0125-preview, gpt-4-1106-preview, gpt-4-turbo, gpt-4-turbo-2024-04-09, gpt-4-turbo-preview, gpt-3.5-turbo, gpt-3.5-turbo-0125, gpt-3.5-turbo-0301, gpt-3.5-turbo-0613, gpt-3.5-turbo-0613, gpt-3.5-turbo-16k, gpt-3.5-turbo-16k-0613]
  -f, --file <FILE>        Initial context file
  -h, --help               Print help
  -V, --version            Print version
```

## installation

```bash
cargo install FerriteChatter
```

## Use in vim
```vim
function! ChatAIWithContext()
    let l:temp_file = tempname()
    execute 'write ' . l:temp_file
    execute 'rightbelow vsplit | terminal fchat -f ' . l:temp_file
endfunction

vnoremap <silent> <C-f> :<C-u>call ChatAIWithContext()<CR>
```

## config file (Optional)
`$HOME/.ferriteconf.yaml` or `$XDG_CONFIG_HOME/.ferriteconf.yaml`


```yaml
openai_api_key: "XXXX"
default_model: "gpt-4o"
```

