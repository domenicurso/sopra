loader_frames=('-' '\' '|' '/')

render_step() {
    local label=$1
    local subtext=$2
    local frame=$3

    if (( animate )); then
        if [[ -n $subtext ]]; then
            printf '\r\033[K%b%s%b %b%s%b %s' \
                "$cyan" "$label" "$reset" "$dim" "$subtext" "$reset" "$frame"
        else
            printf '\r\033[K%b%s%b %s' "$cyan" "$label" "$reset" "$frame"
        fi
    elif [[ -n $subtext ]]; then
        printf '%s: %s\n' "$label" "$subtext"
    else
        printf '%s\n' "$label"
    fi
}

run_step() {
    local label=$1
    shift
    local status=0
    local subtext='starting'
    local frame_index=0
    local line

    step_dir=$(mktemp -d "${TMPDIR:-/tmp}/keel-step.XXXXXX")
    local step_pipe="$step_dir/output"
    mkfifo "$step_pipe"
    : > "$build_log"

    "$@" >"$step_pipe" 2>&1 &
    local pid=$!
    while IFS= read -r line || [[ -n $line ]]; do
        printf '%s\n' "$line" >> "$build_log"
        case $line in
            keel-progress:*)
                subtext=${line#keel-progress:}
                ;;
            patching\ file\ *)
                subtext=${line#patching file }
                subtext=${subtext#\'}
                subtext=${subtext%\'}
                subtext="Patching $subtext"
                ;;
            Patching\ file\ *)
                subtext=${line#Patching file }
                subtext=${subtext#\'}
                subtext=${subtext%\'}
                subtext="Patching $subtext"
                ;;
            *)
                continue
                ;;
        esac

        local frame=${loader_frames[$((frame_index % ${#loader_frames[@]}))]}
        render_step "$label" "$subtext" "$frame"
        frame_index=$((frame_index + 1))
    done < "$step_pipe"
    wait "$pid" || status=$?
    rm -rf -- "$step_dir"
    step_dir=''

    if (( status != 0 )); then
        if (( animate )); then
            printf '\r\033[K%b%s%b\n' "${bold}${red}" "$label failed" "$reset" >&2
        else
            printf '%s failed\n' "$label" >&2
        fi
        cat "$build_log" >&2
        return "$status"
    fi

    if (( animate )); then
        printf '\r\033[K%b%s%b %b%s%b\n' "$cyan" "$label" "$reset" "$green" 'done' "$reset"
    fi
}
