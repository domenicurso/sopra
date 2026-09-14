_keel_completion_stop_pending() {
    if [[ -n $_KEEL_COMPLETION_FD ]]; then
        zle -F "$_KEEL_COMPLETION_FD" 2>/dev/null || true
    fi
    zle keel-native-stop-completion 2>/dev/null || true
    _KEEL_COMPLETION_FD=''
    _KEEL_COMPLETION_PENDING_ID=''
    _KEEL_COMPLETION_PENDING_BUFFER=''
    _KEEL_COMPLETION_PENDING_CURSOR=''
    _KEEL_COMPLETION_PENDING_PWD=''
    _KEEL_COMPLETION_PENDING_CACHE_KEY=''
    _KEEL_COMPLETION_NATIVE_CACHE_KEY=''
}

_keel_completion_command_name() {
    emulate -L zsh
    local -a words

    words=("${(@z)1}")
    REPLY=${words[1]}
}

_keel_completion_prefetch_file_key() {
    emulate -L zsh
    REPLY=${1//[^[:alnum:]_.-]/_}
}

_keel_completion_prefetch_poll() {
    emulate -L zsh

    local command_name=${1:t}
    local key generated_file generated_done arguments_file arguments_done
    local help_file help_done

    [[ -n $_KEEL_COMPLETION_PREFETCH_DIR && -n $command_name ]] || return 0
    _keel_completion_prefetch_file_key "$command_name"
    key=$REPLY
    generated_file="$_KEEL_COMPLETION_PREFETCH_DIR/$key.generated"
    generated_done="$_KEEL_COMPLETION_PREFETCH_DIR/$key.generated.done"
    arguments_file="$_KEEL_COMPLETION_PREFETCH_DIR/$key.arguments"
    arguments_done="$_KEEL_COMPLETION_PREFETCH_DIR/$key.arguments.done"
    help_file="$_KEEL_COMPLETION_PREFETCH_DIR/$key.help"
    help_done="$_KEEL_COMPLETION_PREFETCH_DIR/$key.help.done"

    if [[ -f $generated_file && ${_KEEL_COMPLETION_PREFETCH_GENERATED_READY[$command_name]:-0} != 1 ]]; then
        _KEEL_COMPLETION_PREFETCH_GENERATED[$command_name]=$(<"$generated_file")
        _KEEL_COMPLETION_PREFETCH_GENERATED_READY[$command_name]=1
    fi
    [[ -f $generated_done ]] && _KEEL_COMPLETION_PREFETCH_GENERATED_DONE[$command_name]=1
    if [[ -f $arguments_file && ${_KEEL_COMPLETION_PREFETCH_ARGUMENTS_READY[$command_name]:-0} != 1 ]]; then
        _KEEL_COMPLETION_PREFETCH_ARGUMENTS[$command_name]=$(<"$arguments_file")
        _KEEL_COMPLETION_PREFETCH_ARGUMENTS_READY[$command_name]=1
    fi
    [[ -f $arguments_done ]] && _KEEL_COMPLETION_PREFETCH_ARGUMENTS_DONE[$command_name]=1
    if [[ -f $help_file && ${_KEEL_COMPLETION_PREFETCH_HELP_READY[$command_name]:-0} != 1 ]]; then
        _KEEL_COMPLETION_PREFETCH_HELP[$command_name]=$(<"$help_file")
        _KEEL_COMPLETION_PREFETCH_HELP_READY[$command_name]=1
    fi
    [[ -f $help_done ]] && _KEEL_COMPLETION_PREFETCH_HELP_DONE[$command_name]=1
    if (( ${_KEEL_COMPLETION_PREFETCH_GENERATED_READY[$command_name]:-0} )) &&
        (( ${_KEEL_COMPLETION_PREFETCH_ARGUMENTS_STARTED[$command_name]:-0} != 1 )) &&
        (( ${_KEEL_COMPLETION_PREFETCH_ARGUMENTS_DONE[$command_name]:-0} != 1 )) &&
        [[ ${_KEEL_COMPLETION_PREFETCH_GENERATED[$command_name]} == *'--get-yargs-completions'* ]]; then
        _keel_completion_prefetch_arguments_start "$command_name"
    fi
}

_keel_completion_prefetch_start() {
    emulate -L zsh

    local command_name=${1:t}
    local command_path key prefetch_dir generated_file generated_tmp generated_done
    local help_file help_tmp help_done temp_suffix

    [[ -n $command_name && -n ${commands[$command_name]-} ]] || return 0
    [[ -z ${_KEEL_COMPLETION_PREFETCH_STARTED[$command_name]-} ]] || return 0
    command_path=${commands[$command_name]}
    [[ -x $command_path ]] || return 0

    if [[ -z $_KEEL_COMPLETION_PREFETCH_DIR ]]; then
        prefetch_dir=${TMPDIR:-/tmp}
        _KEEL_COMPLETION_PREFETCH_DIR=$(mktemp -d \
            "${prefetch_dir%/}/keel-completion.XXXXXX" 2>/dev/null) || {
            _KEEL_COMPLETION_PREFETCH_DIR=''
            return 0
        }
        chmod 700 -- "$_KEEL_COMPLETION_PREFETCH_DIR" 2>/dev/null || true
    fi
    _keel_completion_prefetch_file_key "$command_name"
    key=$REPLY
    prefetch_dir=$_KEEL_COMPLETION_PREFETCH_DIR
    generated_file="$prefetch_dir/$key.generated"
    generated_done="$prefetch_dir/$key.generated.done"
    help_file="$prefetch_dir/$key.help"
    help_done="$prefetch_dir/$key.help.done"
    temp_suffix="$$.$RANDOM"
    _KEEL_COMPLETION_PREFETCH_STARTED[$command_name]=1

    (
        {
            generated_tmp="$generated_file.$temp_suffix"
            generated_text=$("$command_path" completion zsh 2>/dev/null) || generated_text=''
            if [[ $generated_text == *'#compdef'* ]]; then
                print -rn -- "$generated_text" >| "$generated_tmp" &&
                    mv -f -- "$generated_tmp" "$generated_file"
            else
                rm -f -- "$generated_tmp"
            fi
            : >| "$generated_done.$temp_suffix" &&
                mv -f -- "$generated_done.$temp_suffix" "$generated_done"
        } &
        {
            help_tmp="$help_file.$temp_suffix"
            help_text=$("$command_path" --help 2>&1) || true
            if [[ -n $help_text ]]; then
                print -rn -- "$help_text" >| "$help_tmp" &&
                    mv -f -- "$help_tmp" "$help_file"
            else
                rm -f -- "$help_tmp"
            fi
            : >| "$help_done.$temp_suffix" &&
                mv -f -- "$help_done.$temp_suffix" "$help_done"
        } &
        wait
    ) &!
}

_keel_completion_prefetch_arguments_start() {
    emulate -L zsh

    local command_name=${1:t}
    local command_path key prefetch_dir arguments_file arguments_tmp arguments_done temp_suffix

    [[ -n $command_name && -n ${commands[$command_name]-} ]] || return 0
    [[ -z ${_KEEL_COMPLETION_PREFETCH_ARGUMENTS_STARTED[$command_name]-} ]] || return 0
    command_path=${commands[$command_name]}
    [[ -x $command_path ]] || return 0
    [[ ${_KEEL_COMPLETION_PREFETCH_GENERATED[$command_name]} == *'--get-yargs-completions'* ]] || return 0
    [[ -n $_KEEL_COMPLETION_PREFETCH_DIR ]] || return 0

    _keel_completion_prefetch_file_key "$command_name"
    key=$REPLY
    prefetch_dir=$_KEEL_COMPLETION_PREFETCH_DIR
    arguments_file="$prefetch_dir/$key.arguments"
    arguments_done="$prefetch_dir/$key.arguments.done"
    temp_suffix="$$.$RANDOM"
    _KEEL_COMPLETION_PREFETCH_ARGUMENTS_STARTED[$command_name]=1

    (
        arguments_tmp="$arguments_file.$temp_suffix"
        arguments_text=$("$command_path" --get-yargs-completions "$command_name" 2>/dev/null) || arguments_text=''
        if [[ -n $arguments_text ]]; then
            print -rn -- "$arguments_text" >| "$arguments_tmp" &&
                mv -f -- "$arguments_tmp" "$arguments_file"
        else
            rm -f -- "$arguments_tmp"
        fi
        : >| "$arguments_done.$temp_suffix" &&
            mv -f -- "$arguments_done.$temp_suffix" "$arguments_done"
    ) &!
}

_keel_completion_prefetched_arguments() {
    emulate -L zsh

    local command_name=${words[1]:t}
    local output=${_KEEL_COMPLETION_PREFETCH_ARGUMENTS[$command_name]-}
    local -a reply

    reply=("${(@f)output}")
    if (( ${#reply} )); then
        _describe 'values' reply
    else
        _default
    fi
}

_keel_completion_prefetch_for_line() {
    emulate -L zsh

    local line_prefix=$1
    local command_prefix candidate command_name
    local -a words matches

    words=("${(@z)line_prefix}")
    command_prefix=${words[1]-}
    [[ ${#command_prefix} -ge 2 && $command_prefix != */* && $command_prefix != -* ]] || return 0
    matches=()
    for candidate in ${(k)commands}; do
        [[ $candidate == "$command_prefix"* ]] || continue
        matches+=($candidate)
        (( ${#matches} > 1 )) && return 0
    done
    (( ${#matches} == 1 )) || return 0
    command_name=$matches[1]
    _keel_completion_prefetch_poll "$command_name"
    _keel_completion_prefetch_start "$command_name"
}

_keel_completion_load_generated() {
    emulate -L zsh
    local command_name=${1:t}
    local generated provider

    if (( $+_comps[$command_name] )); then
        provider=${_comps[$command_name]}
        # Generated CLI providers commonly occupy the conventional
        # _command name as an autoload placeholder.  Load their generated
        # script before treating the completion as fully initialized.
        if [[ $provider != _${command_name} ||
              ${functions[$provider]-} != *'builtin autoload -XU'* ]]; then
            _keel_completion_prefetch_poll "$command_name"
            if (( ${_KEEL_COMPLETION_PREFETCH_ARGUMENTS_READY[$command_name]:-0} )) &&
                [[ ${_KEEL_COMPLETION_PREFETCH_GENERATED[$command_name]-} == *'--get-yargs-completions'* ]]; then
                compdef _keel_completion_prefetched_arguments "$command_name" 2>/dev/null || true
            fi
            return 0
        fi
    fi
    (( $+commands[$command_name] || $+functions[$command_name] )) || return 1
    _keel_completion_prefetch_poll "$command_name"
    if (( ${_KEEL_COMPLETION_PREFETCH_GENERATED_READY[$command_name]:-0} )); then
        generated=${_KEEL_COMPLETION_PREFETCH_GENERATED[$command_name]}
        eval "$generated" 2>/dev/null || return 1
        if (( ${_KEEL_COMPLETION_PREFETCH_ARGUMENTS_READY[$command_name]:-0} )) &&
            [[ $generated == *'--get-yargs-completions'* ]]; then
            compdef _keel_completion_prefetched_arguments "$command_name" 2>/dev/null || true
        fi
        (( $+_comps[$command_name] )) && return 0
    elif (( _KEEL_COMPLETION_CAPTURE )) &&
        (( ! ${_KEEL_COMPLETION_PREFETCH_GENERATED_DONE[$command_name]:-0} )); then
        if generated=$("$command_name" completion zsh 2>/dev/null) &&
            [[ $generated == *'#compdef'* ]]; then
            eval "$generated" 2>/dev/null || return 1
            if (( ${_KEEL_COMPLETION_PREFETCH_ARGUMENTS_READY[$command_name]:-0} )) &&
                [[ $generated == *'--get-yargs-completions'* ]]; then
                compdef _keel_completion_prefetched_arguments "$command_name" 2>/dev/null || true
            fi
            (( $+_comps[$command_name] )) && return 0
        fi
    fi
    _keel_completion_load_help_provider "$command_name"
}

_keel_completion_request() {
    local current_buffer=$BUFFER
    local current_cursor=$CURSOR
    local current_pwd=$PWD
    local request completion_widget cache_key cached line_prefix

    _KEEL_COMPLETION_LATEST_BUFFER=$current_buffer
    _KEEL_COMPLETION_LATEST_CURSOR=$current_cursor
    _KEEL_COMPLETION_LATEST_PWD=$current_pwd

    if [[ -z $current_buffer ]]; then
        _keel_completion_stop_pending
        _KEEL_COMPLETION_LAST_ID=''
        _KEEL_COMPLETION_LAST_BUFFER=''
        _KEEL_COMPLETION_LAST_CURSOR=''
        _KEEL_COMPLETION_LAST_PWD=''
        _KEEL_COMPLETION_LATEST_BUFFER=''
        _KEEL_COMPLETION_LATEST_CURSOR=0
        _KEEL_COMPLETION_LATEST_PWD=''
        return 0
    fi

    line_prefix=${current_buffer[1,current_cursor]}
    _keel_completion_prefetch_for_line "$line_prefix"
    whence -w compdef >/dev/null 2>&1 || return 0
    whence -w _main_complete >/dev/null 2>&1 || return 0

    completion_widget=${(k)widgets[(r)completion:.complete-word:_main_complete]}
    [[ -n $completion_widget ]] || return 0
    _keel_completion_broad_context "$current_buffer" "$current_cursor"
    cache_key="${current_pwd}"$'\x1f'"${completion_widget}"$'\x1f'"${_KEEL_COMPLETION_BROAD_BUFFER}"$'\x1f'"${_KEEL_COMPLETION_BROAD_CURSOR}"$'\x1f'"${_KEEL_COMPLETION_LONG_OPTION_PROBE}"

    if [[ $current_buffer == "$_KEEL_COMPLETION_PENDING_BUFFER" &&
          $current_cursor == "$_KEEL_COMPLETION_PENDING_CURSOR" &&
          $current_pwd == "$_KEEL_COMPLETION_PENDING_PWD" &&
          -n $_KEEL_COMPLETION_FD ]]; then
        return 0
    fi

    # Keep one provider request alive while the user edits the same token;
    # the result is broad enough for every fuzzy query in this context.
    if [[ -n $_KEEL_COMPLETION_FD &&
          $cache_key == "$_KEEL_COMPLETION_PENDING_CACHE_KEY" ]]; then
        return 0
    fi
    if [[ $current_buffer == "$_KEEL_COMPLETION_LAST_BUFFER" &&
          $current_cursor == "$_KEEL_COMPLETION_LAST_CURSOR" &&
          $current_pwd == "$_KEEL_COMPLETION_LAST_PWD" &&
          $cache_key == "$_KEEL_COMPLETION_NATIVE_CACHE_KEY" ]]; then
        return 0
    fi

    _keel_completion_stop_pending
    _KEEL_COMPLETION_LAST_ID=''
    _KEEL_COMPLETION_LAST_BUFFER=''
    _KEEL_COMPLETION_LAST_CURSOR=''
    _KEEL_COMPLETION_LAST_PWD=''

    (( _KEEL_COMPLETION_REQUEST++ ))
    request=$_KEEL_COMPLETION_REQUEST
    if _keel_completion_cache_get "$cache_key"; then
        cached=$REPLY
        _KEEL_COMPLETION_LAST_ID=$request
        _KEEL_COMPLETION_LAST_BUFFER=$current_buffer
        _KEEL_COMPLETION_LAST_CURSOR=$current_cursor
        _KEEL_COMPLETION_LAST_PWD=$current_pwd
        _KEEL_COMPLETION_RESPONSE_ID=$request
        _KEEL_COMPLETION_RESPONSE_BUFFER=$current_buffer
        _KEEL_COMPLETION_RESPONSE_CURSOR=$current_cursor
        _KEEL_COMPLETION_RESPONSE_PWD=$current_pwd
        _KEEL_COMPLETION_RESPONSE_PAYLOAD=$cached
        _KEEL_COMPLETION_RESPONSE_TENTHS_MS=0
        _KEEL_COMPLETION_RESPONSE_CACHE_KEY=$cache_key
        _KEEL_COMPLETION_APPLY_REDRAW=0
        zle _keel_completion_apply_widget
        _KEEL_COMPLETION_APPLY_REDRAW=1
        return 0
    fi
    if ! zle keel-native-start-completion "$request" "$_KEEL_COMPLETION_LONG_OPTION_PROBE"; then
        return 0
    fi
    _KEEL_COMPLETION_FD=$REPLY
    _KEEL_COMPLETION_PENDING_ID=$request
    _KEEL_COMPLETION_PENDING_BUFFER=$current_buffer
    _KEEL_COMPLETION_PENDING_CURSOR=$current_cursor
    _KEEL_COMPLETION_PENDING_PWD=$current_pwd
    _KEEL_COMPLETION_PENDING_CACHE_KEY=$cache_key
    zle -F "$_KEEL_COMPLETION_FD" _keel_completion_response || _keel_completion_stop_pending
}

_keel_completion_pre_redraw() {
    (( _KEEL_ZSH_HOOKS && ! _KEEL_COMPLETION_CAPTURE )) || return 0
    _keel_completion_prefetch_for_line "${BUFFER[1,CURSOR]}"
    _keel_completion_request
}

_keel_completion_line_init() {
    _KEEL_ABORTING=0
    _keel_completion_stop_pending
    _keel_completion_cache_clear
    _KEEL_COMPLETION_LAST_ID=''
    _KEEL_COMPLETION_LAST_BUFFER=''
    _KEEL_COMPLETION_LAST_CURSOR=''
    _KEEL_COMPLETION_LAST_PWD=''
    _KEEL_COMPLETION_LATEST_BUFFER=''
    _KEEL_COMPLETION_LATEST_CURSOR=0
    _KEEL_COMPLETION_LATEST_PWD=''
    _KEEL_COMPLETION_NATIVE_CACHE_KEY=''
}

_keel_completion_line_finish() {
    _keel_completion_line_init
}
