# FerriteChatter

ChatGPTとターミナル上で会話できるやつ。

名前はChatGPTに考えて貰った。

## Usage
API keyは`OPENAI_API_KEY`に設定

もしくは`$XDG_CONFIG_HOME/ferrite/ferriteconf.yaml`に記載。

あとは実行するだけ

```bash
# Chat形式
# "exit"で終了、"reset"で会話のリセット、"v"でエディターを使用した入力ができる。
$ fchat

# 単発の質問 パイプまたは引数からの入力
$ fask

# Web検索モード（最新情報と引用つき）fchat, fask
$ fchat --web -m gpt-5 "今日のニュース教えて"
`--web` は検索対応モデル（デフォルトは `gpt-5-search-api`）を使用し、引用一覧を `--- Sources ---` に表示します。

# 日英・英日翻訳 パイプまたは引数からの入力
$ ftrans 'hello'
$ cat english_doc.txt | ftrans

# 画像生成
# 引数からプロンプト指定
# (デフォルトサイズは1024x1024、他に1024x1792,1792x1024を指定可能)
# DALL·Eモデル指定
$ fimg -m dalle-2 -n 2 -s 1024x1024 "a futuristic cityscape"
# GPT Imageモデル指定
$ fimg -m gpt-image-1 -n 3 "a surreal landscape"
# パイプ入力からプロンプト指定（モデル選択: 対話式）
echo "cute puppy" | fimg

# 画像編集（GPT Image系のみ）
$ fimg -m gpt-image-1 -i path/to/image.png -n 1 "add red hat"
# マスクを指定して特定領域を編集
$ fimg -m gpt-image-1 -i path/to/image.png -M path/to/mask.png -n 1 "change background to sunset"

# 基本的に共通のオプション
$ ftrans

# 基本的に共通のオプション
# fchatのみ、ファイルからコンテキストを渡せます。

$ fchat -h
ChatGPT CLI

Usage: fchat [OPTIONS]

Options:
  -g, --general <GENERAL>              Open Prompt(General Prompt)
  -k, --key <KEY>                      OpenAI API Key
  -b, --base-url <BASE_URL>            OpenAI API Base URL
  -m, --model <MODEL>                  OpenAI Model [possible values: o3-pro-2025-06-10, gpt-4o-mini-search-preview, gpt-4o-mini-search-preview-2025-03-11, gpt-4-turbo, o3-mini-2025-01-31, gpt-4.1, gpt-4.1-mini-2025-04-14, gpt-5-nano-2025-08-07, gpt-4.1-mini, gpt-4-turbo-2024-04-09, o3-2025-04-16, o4-mini-2025-04-16, gpt-4.1-2025-04-14, gpt-4o-2024-05-13, gpt-4o-search-preview-2025-03-11, gpt-4o-search-preview, gpt-3.5-turbo-16k, o1-mini, o1-mini-2024-09-12, gpt-4o-mini-2024-07-18, o3, o4-mini, gpt-5-chat-latest, o4-mini-deep-research-2025-06-26, gpt-5-nano, gpt-4-turbo-preview, o3-deep-research, chatgpt-4o-latest, gpt-4o-mini-tts, o1-pro-2025-03-19, o1, o1-pro, o4-mini-deep-research, o3-deep-research-2025-06-26, o3-pro, gpt-4o-2024-11-20, gpt-4-0125-preview, gpt-5-mini, gpt-5-mini-2025-08-07, gpt-image-1, gpt-4o-mini, o3-mini, gpt-5, gpt-4.1-nano-2025-04-14, gpt-4.1-nano, gpt-4o-transcribe, gpt-3.5-turbo-instruct, gpt-3.5-turbo-instruct-0914, gpt-4-1106-preview, gpt-5-codex, gpt-4o, gpt-5-2025-08-07, gpt-4o-2024-08-06, o1-2024-12-17, gpt-4, gpt-4-0613, gpt-5-search-api, gpt-3.5-turbo, gpt-3.5-turbo-0125, gpt-4o-transcribe-diarize, gpt-3.5-turbo-1106, gpt-5-search-api-2025-10-14, gpt-5-pro, gpt-5-pro-2025-10-06, gpt-4o-mini-transcribe, gpt-image-1-mini]
  -f, --file <FILE>                    Initial context file
  -r, --response-mode <RESPONSE_MODE>  Response mode (stream or batch) [default: stream] [possible values: stream, batch]
      --web                            Use Web Search API
  -h, --help                           Print help
  -V, --version                        Print version
```

## installation
ビルド時にAPIを叩いて使用可能なモデルを取得しています。インストールする前に`OPENAI_API_KEY`にAPIキーを登録してください。

また、その仕様上モデルの更新には再インストールが必要です

```bash
cargo install FerriteChatter
```

## Use in vim
```vim
function! ChatAIWithContext()
    let l:temp_file = tempname()
    execute "'<,'>write! " . l:temp_file
    execute 'rightbelow vsplit | terminal fchat -f ' . l:temp_file 
    call delete(l:temp_file)
endfunction

function! ChatAIWithFile()
    let l:file_path = resolve(expand('%:p'))
    execute 'rightbelow vsplit | terminal fchat -f ' . l:file_path
endfunction

function! AskAIWithContext()
    let l:temp_file = tempname()
    execute "'<,'>write! " . l:temp_file

    let buf = nvim_create_buf(v:false, v:true)
    call nvim_buf_set_lines(buf, 0, -1, v:true, ['> '])
    let width = 40
    let height = 1

    let opts = {
                \ 'relative': 'editor',
                \ 'width': width,
                \ 'height': height,
                \ 'col': (&columns - width) / 2,
                \ 'row': (&lines - height) / 2,
                \ 'anchor': 'NW',
                \ 'style': 'minimal',
                \ 'border': 'single'
                \ }

    let win = nvim_open_win(buf, v:true, opts)
    let l:user_input = input('> ')
    call nvim_win_close(win, v:true)

    if l:user_input == ''
        echom 'No input provided'
        return
    endif

    let l:command = 'fask -f ' . l:temp_file . ' ' . shellescape(l:user_input)
    let l:result = system(l:command)

    let result_buf = nvim_create_buf(v:false, v:true)
    call nvim_buf_set_lines(result_buf, 0, -1, v:true, split(l:result, "\n"))

    let result_height = min([20, len(split(l:result, "\n"))])
    let result_width = min([80, max(map(split(l:result, "\n"), 'len(v:val)'))])

    let result_opts = {
                \ 'relative': 'editor',
                \ 'width': result_width,
                \ 'height': result_height,
                \ 'col': (&columns - result_width) / 2,
                \ 'row': (&lines - result_height) / 2,
                \ 'anchor': 'NW',
                \ 'style': 'minimal',
                \ 'border': 'single'
                \ }

    call nvim_open_win(result_buf, v:true, result_opts)
    call delete(l:temp_file)
endfunction

function! AskAIWithFile()
    let l:file_path = resolve(expand('%:p'))

    let buf = nvim_create_buf(v:false, v:true)
    call nvim_buf_set_lines(buf, 0, -1, v:true, ['> '])
    let width = 40
    let height = 1

    let opts = {
                \ 'relative': 'editor',
                \ 'width': width,
                \ 'height': height,
                \ 'col': (&columns - width) / 2,
                \ 'row': (&lines - height) / 2,
                \ 'anchor': 'NW',
                \ 'style': 'minimal',
                \ 'border': 'single'
                \ }

    let win = nvim_open_win(buf, v:true, opts)
    let l:user_input = input('> ')
    call nvim_win_close(win, v:true)

    if l:user_input == ''
        echom 'No input provided'
        return
    endif

    let l:command = 'fask ' . shellescape(l:user_input) . ' -f ' . l:file_path
    let l:result = system(l:command)

    let result_buf = nvim_create_buf(v:false, v:true)
    call nvim_buf_set_lines(result_buf, 0, -1, v:true, split(l:result, "\n"))

    let result_height = min([20, len(split(l:result, "\n"))])
    let result_width = min([80, max(map(split(l:result, "\n"), 'len(v:val)'))])

    let result_opts = {
                \ 'relative': 'editor',
                \ 'width': result_width,
                \ 'height': result_height,
                \ 'col': (&columns - result_width) / 2,
                \ 'row': (&lines - result_height) / 2,
                \ 'anchor': 'NW',
                \ 'style': 'minimal',
                \ 'border': 'single'
                \ }

    call nvim_open_win(result_buf, v:true, result_opts)
endfunction

function! GenAI()
    let l:user_input = input('> ')
    let l:command = 'fask -g "provide only the code without markdown format in the output." ' . shellescape(l:user_input)
    let l:output = system(l:command)
    set paste
    execute 'normal i' . l:output
    set nopaste
endfunction

function! GenAIWithContext()
    let l:temp_file = tempname()
    execute "'<,'>write! " . l:temp_file
    let l:user_input = input('> ')
    let l:command = 'fask -g "Please provide only the code in the output." -f ' . l:temp_file . ' ' . shellescape(l:user_input)
    let l:output = system(l:command)
    execute 'set paste'
    execute 'normal i' . l:output
    execute 'set nopaste'
endfunction

function! ReplaceAIWithContext()
    let l:temp_file = tempname()
    execute "'<,'>write! " . l:temp_file
    let l:user_input = input('> ')
    let l:command = 'fask -g "provide only the code without markdown format in the output." -f ' . l:temp_file . ' ' . shellescape(l:user_input)
    let l:output = system(l:command)
    execute "'<,'>delete"
    set paste
    execute "normal! a" . l:output
    set nopaste
    call delete(l:temp_file)
endfunction

vnoremap <silent> <C-f> :<C-u>call ChatAIWithContext()<CR>
vnoremap <silent> <C-a> :<C-u>call AskAIWithContext()<CR>
nnoremap <silent> <C-f> :<C-u>call ChatAIWithFile()<CR>
nnoremap <silent> <C-a> :<C-u>call AskAIWithFile()<CR>
nnoremap <silent> <C-g> :<C-u>call GenAI()<CR>
vnoremap <silent> <C-g> :<C-u>call GenAIWithContext()<CR>
vnoremap <silent> <C-r> :<C-u>call ReplaceAIWithContext()<CR>

tnoremap <Esc> <C-\><C-n>
```

## config file (Optional)
`$HOME/ferrite/ferriteconf.yaml` or `$XDG_CONFIG_HOME/ferrite/ferriteconf.yaml`


```yaml
openai_api_key: "XXXX"
default_model: "gpt-5"
```
