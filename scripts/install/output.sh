loader_frames=('-' '\' '|' '/')

# Keep status events and timed frame events on one stream so one renderer can
# update the text immediately without tying frame animation to build output.
spinner_tick() {
    local label=$1
    local event_pipe=$2
    local frame_index=0
    local subtext='starting'
    local event
    local frame

    while IFS= read -r event; do
        case $event in
            frame)
                frame=${loader_frames[$((frame_index % ${#loader_frames[@]}))]}
                frame_index=$((frame_index + 1))
                render_step "$label" "$subtext" "$frame"
                ;;
            status:*)
                subtext=${event#status:}
                frame=${loader_frames[$((frame_index % ${#loader_frames[@]}))]}
                render_step "$label" "$subtext" "$frame"
                ;;
        esac
    done < "$event_pipe"
}

spinner_clock() {
    local event_pipe=$1
    local ready_file=$2

    exec 5>"$event_pipe"
    : > "$ready_file"
    while :; do
        sleep 0.1
        printf '%s\n' frame >&5
    done
}

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
    local line
    local spinner_pid=''
    local clock_pid=''

    step_dir=$(mktemp -d "${TMPDIR:-/tmp}/keel-step.XXXXXX")
    local step_pipe="$step_dir/output"
    local event_pipe="$step_dir/events"
    local clock_ready="$step_dir/clock-ready"
    mkfifo "$step_pipe"
    if (( animate )); then
        mkfifo "$event_pipe"
        spinner_tick "$label" "$event_pipe" &
        spinner_pid=$!
        spinner_clock "$event_pipe" "$clock_ready" &
        clock_pid=$!
        while [[ ! -f $clock_ready ]]; do
            sleep 0.01
        done
        printf 'status:%s\n' "$subtext" > "$event_pipe"
    fi
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

        if (( animate )); then
            printf 'status:%s\n' "$subtext" > "$event_pipe"
        else
            render_step "$label" "$subtext" ''
        fi
    done < "$step_pipe"
    wait "$pid" || status=$?

    if [[ -n $clock_pid ]]; then
        kill "$clock_pid" 2>/dev/null || true
        wait "$clock_pid" 2>/dev/null || true
    fi
    if [[ -n $spinner_pid ]]; then
        kill "$spinner_pid" 2>/dev/null || true
        wait "$spinner_pid" 2>/dev/null || true
    fi

    rm -f -- "$event_pipe" "$clock_ready"
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
