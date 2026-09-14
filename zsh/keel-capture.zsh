_keel_completion_broad_context() {
    emulate -L zsh

    local current_buffer=$1
    local current_cursor=$2
    local line_prefix token_prefix context_prefix
    local rest_after_cursor token_suffix line_suffix option_prefix
    local path_token path_prefix

    line_prefix=${current_buffer[1,current_cursor]}
    token_prefix=${line_prefix##*[[:space:]]}
    if (( ${#token_prefix} < ${#line_prefix} )); then
        context_prefix=${line_prefix[1,$(( ${#line_prefix} - ${#token_prefix} ))]}
    else
        context_prefix=''
    fi
    rest_after_cursor=${current_buffer[$(( current_cursor + 1 )),-1]}
    token_suffix=${rest_after_cursor%%[[:space:]]*}
    line_suffix=${rest_after_cursor#${token_suffix}}
    typeset -g _KEEL_COMPLETION_LONG_OPTION_PROBE=0
    typeset -g _KEEL_COMPLETION_COMMAND_QUERY=0
    path_token=$token_prefix
    if [[ $path_token == -*=*/* ]]; then
        path_token=${path_token#*=}
    fi
    if [[ $path_token == */* ]]; then
        path_prefix=${token_prefix%/*}/
        typeset -g _KEEL_COMPLETION_BROAD_BUFFER="${context_prefix}${path_prefix}${line_suffix}"
        typeset -g _KEEL_COMPLETION_BROAD_CURSOR=$(( ${#context_prefix} + ${#path_prefix} ))
        return 0
    fi
    if [[ $token_prefix == '~' || $token_prefix == '.' || $token_prefix == '..' ]]; then
        path_prefix=$token_prefix/
        typeset -g _KEEL_COMPLETION_BROAD_BUFFER="${context_prefix}${path_prefix}${line_suffix}"
        typeset -g _KEEL_COMPLETION_BROAD_CURSOR=$(( ${#context_prefix} + ${#path_prefix} ))
        return 0
    fi
    # Keep the first token in command-name completion mode even when it is an
    # exact executable. The current token still needs fuzzy matches, while
    # its argument provider must wait for a separating space.
    if [[ -n $token_prefix && -z $line_suffix &&
          $token_prefix != -* && $token_prefix != */* &&
          -z ${context_prefix//[[:space:]]/} ]]; then
        typeset -g _KEEL_COMPLETION_COMMAND_QUERY=1
    fi
    if [[ $token_prefix == --* ]]; then
        typeset -g _KEEL_COMPLETION_LONG_OPTION_PROBE=1
    fi
    if [[ $token_prefix == --* ]]; then
        option_prefix='--'
    elif [[ $token_prefix == -* ]]; then
        option_prefix='-'
    else
        option_prefix=''
    fi
    typeset -g _KEEL_COMPLETION_BROAD_BUFFER="${context_prefix}${option_prefix}${line_suffix}"
    typeset -g _KEEL_COMPLETION_BROAD_CURSOR=$(( ${#context_prefix} + ${#option_prefix} ))
}

_keel_completion_path_fuzzy_match() {
    emulate -L zsh

    local needle=${(L)1}
    local haystack=${(L)2}
    local character
    integer needle_index=1 haystack_index=1

    while (( needle_index <= ${#needle} )); do
        character=${needle[$needle_index]}
        while (( haystack_index <= ${#haystack} )) &&
            [[ ${haystack[$haystack_index]} != "$character" ]]; do
            (( haystack_index++ ))
        done
        (( haystack_index <= ${#haystack} )) || return 1
        (( needle_index++ ))
        (( haystack_index++ ))
    done
}

_keel_completion_path_kind_for_context() {
    emulate -L zsh

    local current_buffer=$1
    local current_cursor=$2
    local line_prefix=${current_buffer[1,current_cursor]}
    local token_prefix=${line_prefix##*[[:space:]]}
    local command_name previous option provider body key map_kind positional_kind
    local field=$'\x1f'
    local -a words
    integer positional_index

    typeset -g _KEEL_COMPLETION_PATH_KIND=both
    typeset -g _KEEL_COMPLETION_PATH_ARGUMENT=0
    words=("${(@z)line_prefix}")
    command_name=${words[1]:t}
    [[ -n $command_name ]] || return 0
    if [[ -n $token_prefix ]]; then
        previous=${words[-2]-}
    else
        previous=${words[-1]-}
    fi
    option=$previous
    [[ $token_prefix == *=* ]] && option=${token_prefix%%=*}
    key="${command_name}${field}${option}"
    map_kind=${_KEEL_HELP_OPTION_KINDS[$key]-}
    case $map_kind in
        directories|files|both)
            typeset -g _KEEL_COMPLETION_PATH_KIND=$map_kind
            typeset -g _KEEL_COMPLETION_PATH_ARGUMENT=1
            return 0
            ;;
    esac

    if [[ -n $token_prefix ]]; then
        positional_index=${#words}-1
    else
        positional_index=${#words}
    fi
    _keel_help_positional_kind "$command_name" "$positional_index"
    positional_kind=$REPLY
    case $positional_kind in
        directories|files|both)
            typeset -g _KEEL_COMPLETION_PATH_KIND=$positional_kind
            typeset -g _KEEL_COMPLETION_PATH_ARGUMENT=1
            return 0
            ;;
    esac

    case $command_name in
        cd|chdir|pushd)
            typeset -g _KEEL_COMPLETION_PATH_KIND=directories
            typeset -g _KEEL_COMPLETION_PATH_ARGUMENT=1
            return 0
            ;;
    esac

    provider=${_comps[$command_name]-}
    body=${functions[$provider]-}
    if [[ $body == *'_directories'* || $body == *'_files -/'* ]] ||
        { [[ $body == *'_path_files'* ]] && [[ $body == *'-/'* ]]; }; then
        typeset -g _KEEL_COMPLETION_PATH_KIND=directories
        typeset -g _KEEL_COMPLETION_PATH_ARGUMENT=1
    elif [[ $body == *'_files'* || $body == *'_path_files'* ]]; then
        typeset -g _KEEL_COMPLETION_PATH_KIND=both
        typeset -g _KEEL_COMPLETION_PATH_ARGUMENT=1
    fi
}

_keel_completion_path_is_exact() {
    emulate -L zsh
    setopt localoptions nullglob globdots

    local path=$1
    local rest cursor component entry matched

    [[ -n $path ]] || return 0
    if [[ $path == /* ]]; then
        cursor=/
        rest=${path#/}
    elif [[ $path == '~' || $path == '~/'* ]]; then
        cursor=$HOME
        rest=${path#\~}
        rest=${rest#/}
    else
        cursor=$PWD
        rest=$path
    fi

    for component in "${(@s:/:)rest}"; do
        [[ -n $component && $component != . ]] || continue
        if [[ $component == .. ]]; then
            cursor="$cursor/.."
            continue
        fi
        matched=''
        for entry in "$cursor"/*(N); do
            [[ ${entry:t} == "$component" ]] || continue
            matched=$entry
            break
        done
        [[ -n $matched ]] || return 1
        cursor=$matched
    done
    [[ -d $cursor ]]
}

_keel_completion_path_parent_is_fuzzy() {
    emulate -L zsh

    local current_buffer=$1
    local current_cursor=$2
    local line_prefix=${current_buffer[1,current_cursor]}
    local token_prefix=${line_prefix##*[[:space:]]}
    local path_token=$token_prefix
    local parent

    [[ $path_token == */* ]] || return 1
    if [[ $path_token == -*=*/* ]]; then
        path_token=${path_token#*=}
    fi
    parent=${path_token%/*}
    [[ -n $parent ]] || parent=/
    ! _keel_completion_path_is_exact "$parent"
}

_keel_completion_fuzzy_path_candidates() {
    emulate -L zsh
    setopt localoptions nullglob globdots

    local parent=$1
    local leaf=$2
    local kind=${3:-both}
    local display_prefix rest base entry name candidate
    local branch branch_name component flat_index_value
    integer index ordinal max_ordinal flat_index
    local -a requested branch_paths branch_names next_paths next_names entries
    local -a entry_paths entry_names branch_counts
    local -A entry_by_branch_ordinal

    typeset -ga _KEEL_COMPLETION_FUZZY_PATH_CANDIDATES
    _KEEL_COMPLETION_FUZZY_PATH_CANDIDATES=()
    while [[ $parent == */ && $parent != / ]]; do
        parent=${parent%/}
    done

    if [[ $parent == /* ]]; then
        display_prefix=/
        rest=${parent#/}
        base=/
    elif [[ $parent == '~' || $parent == '~/'* ]]; then
        display_prefix='~/'
        rest=${parent#\~}
        rest=${rest#/}
        base=$HOME
    elif [[ $parent == . || $parent == ./* ]]; then
        display_prefix='./'
        rest=${parent#./}
        [[ $parent == . ]] && rest=''
        base=$PWD
    elif [[ $parent == .. || $parent == ../* ]]; then
        display_prefix=''
        rest=$parent
        base=$PWD
        while [[ $rest == .. || $rest == ../* ]]; do
            display_prefix+='../'
            base="$base/.."
            if [[ $rest == .. ]]; then
                rest=''
                break
            fi
            rest=${rest#../}
        done
    else
        display_prefix=''
        rest=$parent
        base=$PWD
    fi

    requested=("${(@s:/:)rest}")
    requested=("${(@)requested:#}")
    branch_paths=("$base")
    branch_names=('')
    for component in "${requested[@]}"; do
        [[ -n $component ]] || continue
        next_paths=()
        next_names=()
        for index in {1..${#branch_paths}}; do
            branch=${branch_paths[index]}
            branch_name=${branch_names[index]}
            if [[ $component == . ]]; then
                next_paths+=("$branch")
                next_names+=("$branch_name")
                continue
            fi
            if [[ $component == .. ]]; then
                next_paths+=("$branch/..")
                next_names+=("${branch_name}${branch_name:+/}..")
                continue
            fi
            for entry in "$branch"/*(N); do
                [[ -d $entry ]] || continue
                name=${entry:t}
                _keel_completion_path_fuzzy_match "$component" "$name" || continue
                next_paths+=("$entry")
                next_names+=("${branch_name}${branch_name:+/}$name")
                (( ${#next_paths} >= 64 )) && break
            done
            (( ${#next_paths} >= 64 )) && break
        done
        branch_paths=("${next_paths[@]}")
        branch_names=("${next_names[@]}")
        (( ${#branch_paths} )) || return 0
    done

    entry_paths=()
    entry_names=()
    branch_counts=()
    max_ordinal=0
    for index in {1..${#branch_paths}}; do
        branch=${branch_paths[index]}
        entries=("$branch"/*(N))
        ordinal=0
        for entry in "${entries[@]}"; do
            name=${entry:t}
            [[ $name == .* && $leaf != .* ]] && continue
            [[ -z $leaf ]] || _keel_completion_path_fuzzy_match "$leaf" "$name" || continue
            if [[ $kind == directories && ! -d $entry ]]; then
                continue
            fi
            if [[ $kind == files && -d $entry ]]; then
                continue
            fi
            (( ordinal++ ))
            entry_paths+=("$entry")
            entry_names+=("$name")
            branch_counts[index]=$ordinal
            entry_by_branch_ordinal["$index:$ordinal"]=${#entry_paths}
            (( ordinal > max_ordinal )) && max_ordinal=ordinal
            (( ordinal >= 256 )) && break
        done
    done

    # Interleave branches before applying the global cap. This keeps a broad
    # fuzzy parent useful when one branch contains a very large directory.
    for (( ordinal = 1; ordinal <= max_ordinal; ordinal++ )); do
        for index in {1..${#branch_paths}}; do
            flat_index_value=${entry_by_branch_ordinal["$index:$ordinal"]-}
            [[ -n $flat_index_value ]] || continue
            flat_index=$flat_index_value
            branch_name=${branch_names[index]}
            name=${entry_names[flat_index]}
            candidate="${display_prefix}${branch_name}${branch_name:+/}$name"
            [[ -d ${entry_paths[flat_index]} ]] && candidate+=/
            _KEEL_COMPLETION_FUZZY_PATH_CANDIDATES+=("$candidate")
            (( ${#_KEEL_COMPLETION_FUZZY_PATH_CANDIDATES} >= 256 )) && break 2
        done
    done
    typeset -Ua _KEEL_COMPLETION_FUZZY_PATH_CANDIDATES
}

_keel_completion_fuzzy_path_complete() {
    emulate -L zsh

    local current_buffer=$1
    local current_cursor=$2
    local line_prefix=${current_buffer[1,current_cursor]}
    local token_prefix=${line_prefix##*[[:space:]]}
    local path_token=$token_prefix
    local insertion_prefix=''
    local parent leaf
    integer path_like=0
    local saved_prefix saved_iprefix saved_suffix saved_isuffix
    integer compadd_status
    local -a candidates

    if [[ $path_token == -*=* ]]; then
        insertion_prefix=${path_token%%=*}=
        path_token=${path_token#*=}
    fi
    _keel_completion_path_kind_for_context "$current_buffer" "$current_cursor"
    if [[ $path_token == */* ]]; then
        path_like=1
        parent=${path_token%/*}
        leaf=${path_token##*/}
        [[ -n $parent ]] || parent=/
    elif [[ $path_token == . || $path_token == .. || $path_token == '~' ]]; then
        path_like=1
        parent=$path_token
        leaf=''
    else
        parent=''
        leaf=$path_token
    fi
    (( _KEEL_COMPLETION_PATH_ARGUMENT || path_like )) || return 0
    _keel_completion_fuzzy_path_candidates "$parent" "$leaf" \
        "$_KEEL_COMPLETION_PATH_KIND"
    candidates=("${_KEEL_COMPLETION_FUZZY_PATH_CANDIDATES[@]}")
    (( ${#candidates} )) || return 0
    if [[ -n $insertion_prefix ]]; then
        candidates=("${(@)candidates/#/$insertion_prefix}")
    fi
    saved_prefix=$PREFIX
    saved_iprefix=$IPREFIX
    saved_suffix=$SUFFIX
    saved_isuffix=$ISUFFIX
    PREFIX=''
    IPREFIX=''
    SUFFIX=''
    ISUFFIX=''
    compadd -U -f -- "${candidates[@]}"
    compadd_status=$?
    PREFIX=$saved_prefix
    IPREFIX=$saved_iprefix
    SUFFIX=$saved_suffix
    ISUFFIX=$saved_isuffix
    return $compadd_status
}

_keel_completion_prepare_provider() {
    emulate -L zsh

    local command_name=$1
    local provider=${_comps[$command_name]-}

    # macOS grep advertises the long options that _grep hides for its BSD
    # variant.  Keel asks for a complete long-option set, so keep that
    # provider's own descriptions instead of truncating the API result.
    if [[ $OSTYPE == darwin* && $command_name == grep && $provider == _grep ]]; then
        typeset -gA _cmd_variant
        _cmd_variant[$command_name]=gnu
    fi
}

_keel_completion_capture_sync() {
    emulate -L zsh
    setopt localoptions no_monitor

    local request=$1
    local completion_widget
    local current_buffer current_cursor broad_buffer broad_cursor command_name path_mode
    local -a command_words
    local -F started finished
    integer elapsed_tenths_ms=0
    typeset -g _KEEL_COMPLETION_CAPTURE=1

    _keel_completion_capture_post() {
        local fuzzy_path=0

        _keel_completion_path_parent_is_fuzzy \
            "$_KEEL_COMPLETION_CAPTURE_BUFFER" "$_KEEL_COMPLETION_CAPTURE_CURSOR" &&
            fuzzy_path=1
        _keel_completion_path_kind_for_context \
            "$_KEEL_COMPLETION_CAPTURE_BUFFER" "$_KEEL_COMPLETION_CAPTURE_CURSOR"
        if (( _KEEL_COMPLETION_PATH_ARGUMENT && ! fuzzy_path )); then
            case $_KEEL_COMPLETION_PATH_KIND in
                directories)
                    _directories
                    ;;
                files)
                    _files -f
                    ;;
                both)
                    _files
                    ;;
            esac
        fi
        _keel_completion_fuzzy_path_complete \
            "$_KEEL_COMPLETION_CAPTURE_BUFFER" "$_KEEL_COMPLETION_CAPTURE_CURSOR"
        compstate[insert]=''
        compstate[list]=''
    }
    local -a saved_comppostfuncs
    saved_comppostfuncs=("${comppostfuncs[@]}")
    comppostfuncs=(_keel_completion_capture_post)
    completion_widget=${(k)widgets[(r)completion:.complete-word:_main_complete]}

    current_buffer=$BUFFER
    current_cursor=$CURSOR
    typeset -g _KEEL_COMPLETION_CAPTURE_BUFFER=$current_buffer
    typeset -g _KEEL_COMPLETION_CAPTURE_CURSOR=$current_cursor
    if [[ -n $completion_widget ]]; then
        if [[ -n ${EPOCHREALTIME:-} ]]; then
            started=$EPOCHREALTIME
        else
            started=$SECONDS
        fi
        # Capture a broad result set without the active token so the core
        # fuzzy matcher can handle non-prefix queries.
        _keel_completion_broad_context "$current_buffer" "$current_cursor"
        broad_buffer=$_KEEL_COMPLETION_BROAD_BUFFER
        broad_cursor=$_KEEL_COMPLETION_BROAD_CURSOR
        # The broad buffer intentionally removes the active command token
        # while it is being typed, so derive the provider name from the real
        # line rather than from the context used for completion capture.
        command_words=("${(@z)current_buffer}")
        command_name=${command_words[1]-}
        command_name=${command_name:t}
        if (( ! _KEEL_COMPLETION_COMMAND_QUERY )); then
            _keel_completion_load_generated "$command_name"
            _keel_help_load "$command_name" >/dev/null 2>&1 || true
        fi
        _keel_completion_path_kind_for_context "$current_buffer" "$current_cursor"
        case $_KEEL_COMPLETION_PATH_KIND in
            directories)
                path_mode=1
                ;;
            files)
                path_mode=2
                ;;
            *)
                path_mode=0
                ;;
        esac
        zle keel-native-set-completion-path-mode "$path_mode" >/dev/null 2>&1 || true
        _keel_completion_prepare_provider "$command_name"
        zstyle ':completion:*' verbose yes
        zstyle ':completion:*' list-grouped no
        BUFFER=$broad_buffer
        CURSOR=$broad_cursor
        zle -- "$completion_widget" >/dev/null 2>&1
        comppostfuncs=("${saved_comppostfuncs[@]}")
        BUFFER=$current_buffer
        CURSOR=$current_cursor
        if [[ -n ${EPOCHREALTIME:-} ]]; then
            finished=$EPOCHREALTIME
            elapsed_tenths_ms=$(( (finished - started) * 10000 ))
        else
            elapsed_tenths_ms=$(( (SECONDS - started) * 10000 ))
        fi
    fi
    (( elapsed_tenths_ms < 0 )) && elapsed_tenths_ms=0

    zle keel-native-finish-capture "$request" "$elapsed_tenths_ms"
}
