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
            return 0
            ;;
    esac

    case $command_name in
        cd|chdir|pushd)
            typeset -g _KEEL_COMPLETION_PATH_KIND=directories
            return 0
            ;;
    esac

    provider=${_comps[$command_name]-}
    body=${functions[$provider]-}
    if [[ $body == *'_directories'* || $body == *'_files -/'* ]] ||
        { [[ $body == *'_path_files'* ]] && [[ $body == *'-/'* ]]; }; then
        typeset -g _KEEL_COMPLETION_PATH_KIND=directories
    elif [[ $body == *'_files'* || $body == *'_path_files'* ]]; then
        typeset -g _KEEL_COMPLETION_PATH_KIND=both
    fi
}

_keel_completion_fuzzy_path_candidates() {
    emulate -L zsh
    setopt localoptions nullglob globdots

    local parent=$1
    local leaf=$2
    local kind=${3:-both}
    local display_prefix rest base entry name candidate
    local branch branch_name component
    integer index
    local -a requested branch_paths branch_names next_paths next_names entries

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

    if [[ -z $leaf ]]; then
        # A trailing slash means the working segment is empty. If the
        # preceding segment was fuzzy, complete that segment itself instead
        # of descending into every matched directory.
        for index in {1..${#branch_paths}}; do
            branch=${branch_paths[index]}
            branch_name=${branch_names[index]}
            [[ -d $branch ]] || continue
            candidate="${display_prefix}${branch_name}${branch_name:+/}"
            [[ -n $candidate ]] || continue
            _KEEL_COMPLETION_FUZZY_PATH_CANDIDATES+=("$candidate")
            (( ${#_KEEL_COMPLETION_FUZZY_PATH_CANDIDATES} >= 256 )) && break
        done
        typeset -Ua _KEEL_COMPLETION_FUZZY_PATH_CANDIDATES
        return 0
    fi

    for index in {1..${#branch_paths}}; do
        branch=${branch_paths[index]}
        branch_name=${branch_names[index]}
        entries=("$branch"/*(N))
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
            candidate="${display_prefix}${branch_name}${branch_name:+/}$name"
            [[ -d $entry ]] && candidate+=/
            _KEEL_COMPLETION_FUZZY_PATH_CANDIDATES+=("$candidate")
            (( ${#_KEEL_COMPLETION_FUZZY_PATH_CANDIDATES} >= 256 )) && break
        done
        (( ${#_KEEL_COMPLETION_FUZZY_PATH_CANDIDATES} >= 256 )) && break
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
    local parent leaf expanded_parent candidate
    local -a candidates

    if [[ $path_token == -*=*/* ]]; then
        insertion_prefix=${path_token%%=*}=
        path_token=${path_token#*=}
    fi
    [[ $path_token == */* ]] || return 0
    parent=${path_token%/*}
    leaf=${path_token##*/}
    [[ -n $parent ]] || parent=/
    expanded_parent=$parent
    [[ $expanded_parent == '~/'* ]] && expanded_parent=${~expanded_parent}
    [[ -d $expanded_parent ]] && return 0

    _keel_completion_path_kind_for_context "$current_buffer" "$current_cursor"
    _keel_completion_fuzzy_path_candidates "$parent" "$leaf" \
        "$_KEEL_COMPLETION_PATH_KIND"
    candidates=("${_KEEL_COMPLETION_FUZZY_PATH_CANDIDATES[@]}")
    (( ${#candidates} )) || return 0
    if [[ -n $insertion_prefix ]]; then
        candidates=("${(@)candidates/#/$insertion_prefix}")
    fi
    compadd -U -f -- "${candidates[@]}"
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
    local current_buffer current_cursor broad_buffer broad_cursor command_name
    local -F started finished
    integer elapsed_tenths_ms=0
    typeset -g _KEEL_COMPLETION_CAPTURE=1

    _keel_completion_capture_post() {
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
        command_name=${${(@z)broad_buffer}[1]}
        command_name=${command_name:t}
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
