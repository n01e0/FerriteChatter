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
default_model: "gpt-4o"
```

