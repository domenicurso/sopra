typeset -gA _KEEL_COMPLETION_CACHE

_keel_completion_cache_clear() {
    _KEEL_COMPLETION_CACHE=()
}

_keel_completion_cache_get() {
    local key=$1
    if (( ${+_KEEL_COMPLETION_CACHE[$key]} )); then
        REPLY=${_KEEL_COMPLETION_CACHE[$key]}
        return 0
    fi
    return 1
}

_keel_completion_cache_put() {
    local key=$1
    local payload=$2
    _KEEL_COMPLETION_CACHE[$key]=$payload
}
