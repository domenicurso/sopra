#include "keel_zsh_module_internal.h"

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <locale.h>
#include <poll.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <sys/time.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

enum {
    KEEL_TERMINAL_COLOR_QUERY_TIMEOUT_MS = 100,
    KEEL_CURSOR_BLINK_PERIOD_MS = 1800,
    KEEL_CURSOR_ACTIVITY_HOLD_MS = 300,
    KEEL_CURSOR_MIN_OPACITY_PERCENT = 40,
    KEEL_CURSOR_ANIMATION_INTERVAL_NS = 16 * 1000 * 1000,
};

typedef struct {
    unsigned char background[3];
    unsigned char cursor[3];
    int have_background;
    int have_cursor;
} KeelTerminalColors;

static KeelTerminalColors terminal_colors;
static int terminal_colors_queried;
static struct timeval cursor_blink_started;
static int cursor_style_active;
static unsigned char fake_cursor_text[MB_LEN_MAX] = {' '};
static size_t fake_cursor_text_length = 1;
static int fake_cursor_cell_active;
static pid_t cursor_animation_pid = -1;
static int cursor_animation_fd = -1;

static int write_all(const unsigned char *data, size_t length)
{
    while (length > 0) {
        ssize_t written = write(SHTTY, data, length);
        if (written < 0 && errno == EINTR)
            continue;
        if (written <= 0)
            return 0;
        data += written;
        length -= (size_t)written;
    }
    return 1;
}

static int hex_digit(unsigned char value)
{
    if (value >= '0' && value <= '9')
        return value - '0';
    if (value >= 'a' && value <= 'f')
        return value - 'a' + 10;
    if (value >= 'A' && value <= 'F')
        return value - 'A' + 10;
    return -1;
}

static int parse_hex_component(const unsigned char *value, size_t length,
                               unsigned char *output)
{
    unsigned int parsed = 0;
    unsigned int maximum;
    size_t index;

    if (length == 0 || length > 4)
        return 0;
    for (index = 0; index < length; index++) {
        int digit = hex_digit(value[index]);
        if (digit < 0)
            return 0;
        parsed = (parsed << 4) | (unsigned int)digit;
    }
    maximum = (1u << (length * 4)) - 1u;
    *output = (unsigned char)((parsed * 255u + maximum / 2u) / maximum);
    return 1;
}

static int parse_rgb_value(const unsigned char *value, size_t length,
                           unsigned char output[3])
{
    size_t index;

    while (length > 0 && (value[length - 1] == ' ' || value[length - 1] == '\t'))
        length--;
    if (length >= 4 && memcmp(value, "rgb:", 4) == 0) {
        value += 4;
        length -= 4;
        for (index = 0; index < 3; index++) {
            const unsigned char *separator = memchr(value, '/', length);
            size_t component_length = separator == NULL ? length :
                                      (size_t)(separator - value);
            if (!parse_hex_component(value, component_length, &output[index]))
                return 0;
            if (separator == NULL)
                return index == 2;
            length -= component_length + 1;
            value = separator + 1;
        }
        return 0;
    }
    if (length == 7 && value[0] == '#') {
        for (index = 0; index < 3; index++) {
            if (!parse_hex_component(value + 1 + index * 2, 2, &output[index]))
                return 0;
        }
        return 1;
    }
    return 0;
}

static int parse_osc_color(const unsigned char *buffer, size_t length, int code,
                           unsigned char output[3])
{
    unsigned char prefix[8];
    size_t prefix_length;
    size_t index;

    prefix_length = (size_t)snprintf((char *)prefix, sizeof(prefix), "\033]%d;", code);
    if (prefix_length >= sizeof(prefix))
        return 0;
    for (index = 0; index + prefix_length <= length; index++) {
        size_t end;
        if (memcmp(buffer + index, prefix, prefix_length) != 0)
            continue;
        for (end = index + prefix_length; end < length; end++) {
            if (buffer[end] == '\a' ||
                (buffer[end] == '\033' && end + 1 < length && buffer[end + 1] == '\\'))
                return parse_rgb_value(buffer + index + prefix_length,
                                       end - index - prefix_length, output);
        }
    }
    return 0;
}

static unsigned long long elapsed_milliseconds_since(const struct timeval *started)
{
    struct timeval now;
    long long seconds;
    long microseconds;

    (void)gettimeofday(&now, NULL);
    seconds = (long long)now.tv_sec - (long long)started->tv_sec;
    microseconds = now.tv_usec - started->tv_usec;
    if (microseconds < 0) {
        seconds--;
        microseconds += 1000000;
    }
    if (seconds < 0)
        return 0;
    return (unsigned long long)seconds * 1000u + (unsigned long long)microseconds / 1000u;
}

static double smoothstep(double value)
{
    if (value <= 0.0)
        return 0.0;
    if (value >= 1.0)
        return 1.0;
    return value * value * (3.0 - 2.0 * value);
}

static double cursor_opacity(void)
{
    unsigned long long elapsed = elapsed_milliseconds_since(&cursor_blink_started);
    double visibility;

    if (elapsed < KEEL_CURSOR_ACTIVITY_HOLD_MS)
        return 1.0;
    elapsed -= KEEL_CURSOR_ACTIVITY_HOLD_MS;
    double phase = (double)(elapsed % KEEL_CURSOR_BLINK_PERIOD_MS) /
                   (double)KEEL_CURSOR_BLINK_PERIOD_MS;
    double half_cycle = phase * 2.0;

    if (half_cycle <= 1.0)
        visibility = 1.0 - smoothstep(half_cycle);
    else
        visibility = smoothstep(half_cycle - 1.0);
    return (double)KEEL_CURSOR_MIN_OPACITY_PERCENT / 100.0 +
           visibility * (1.0 - (double)KEEL_CURSOR_MIN_OPACITY_PERCENT / 100.0);
}

static unsigned char blend_channel(unsigned char background, unsigned char cursor,
                                   double opacity)
{
    double value = (double)background +
                   ((double)cursor - (double)background) * opacity;
    if (value <= 0.0)
        return 0;
    if (value >= 255.0)
        return 255;
    return (unsigned char)(value + 0.5);
}

static void write_cursor_color(const unsigned char color[3])
{
    char sequence[64];
    int length = snprintf((char *)sequence, sizeof(sequence),
                          "\033]12;#%02x%02x%02x\033\\",
                          color[0], color[1], color[2]);
    if (length > 0 && (size_t)length < sizeof(sequence))
        (void)write_all((const unsigned char *)sequence, (size_t)length);
}

static void capture_fake_cursor_text(void)
{
    mbstate_t state;
    size_t length;

    fake_cursor_text[0] = ' ';
    fake_cursor_text_length = 1;
    if (zleline == NULL || zlecs < 0 || zlecs >= zlell)
        return;
    memset(&state, 0, sizeof(state));
    length = wcrtomb((char *)fake_cursor_text, zleline[zlecs], &state);
    if (length == (size_t)-1 || length == 0 || length > sizeof(fake_cursor_text)) {
        fake_cursor_text[0] = ' ';
        fake_cursor_text_length = 1;
        return;
    }
    fake_cursor_text_length = length;
}

static void restore_fake_cursor_cell(void)
{
    static const unsigned char save_current_cursor[] = "\0337";
    static const unsigned char restore_fake_cursor[] = "\033[u";
    static const unsigned char restore_current_cursor[] = "\0338";

    if (!fake_cursor_cell_active || SHTTY < 0)
        return;
    (void)write_all(save_current_cursor, sizeof(save_current_cursor) - 1);
    (void)write_all(restore_fake_cursor, sizeof(restore_fake_cursor) - 1);
    (void)write_all((const unsigned char *)"\033[0m", 4);
    (void)write_all(fake_cursor_text, fake_cursor_text_length);
    (void)write_all(restore_current_cursor, sizeof(restore_current_cursor) - 1);
    fake_cursor_cell_active = 0;
}

static void write_fake_cursor_cell(void)
{
    static const unsigned char fallback[] = "\033[s\033[7m \033[u";
    unsigned char color[3];
    double opacity;
    int length;
    char sequence[128];
    size_t index;

    if (SHTTY < 0)
        return;
    if (!terminal_colors.have_background || !terminal_colors.have_cursor) {
        (void)write_all(fallback, sizeof(fallback) - 1);
        fake_cursor_cell_active = 1;
        return;
    }
    opacity = cursor_opacity();
    for (index = 0; index < 3; index++)
        color[index] = blend_channel(terminal_colors.background[index],
                                     terminal_colors.cursor[index], opacity);
    length = snprintf(sequence, sizeof(sequence),
                      "\033[s\033[48;2;%u;%u;%um \033[u",
                      color[0], color[1], color[2]);
    if (length > 0 && (size_t)length < sizeof(sequence)) {
        (void)write_all((const unsigned char *)sequence, (size_t)length);
        fake_cursor_cell_active = 1;
    }
}

void keel_query_terminal_colors(void)
{
    static const unsigned char query[] = "\033]11;?\033\\\033]12;?\033\\";
    unsigned char response[1024];
    size_t response_length = 0;
    int flags;
    int original_flags;
    int elapsed = 0;
    struct timeval query_started;

    if (terminal_colors_queried)
        return;
    terminal_colors_queried = 1;
    memset(&terminal_colors, 0, sizeof(terminal_colors));
    if (SHTTY < 0 || !write_all(query, sizeof(query) - 1))
        return;
    (void)gettimeofday(&query_started, NULL);

    original_flags = fcntl(SHTTY, F_GETFL, 0);
    if (original_flags < 0 || fcntl(SHTTY, F_SETFL, original_flags | O_NONBLOCK) < 0)
        return;
    while (elapsed < KEEL_TERMINAL_COLOR_QUERY_TIMEOUT_MS &&
           (!terminal_colors.have_background || !terminal_colors.have_cursor)) {
        struct pollfd descriptor = {.fd = SHTTY, .events = POLLIN};
        unsigned char chunk[256];
        int wait = KEEL_TERMINAL_COLOR_QUERY_TIMEOUT_MS - elapsed;
        int result = poll(&descriptor, 1, wait);
        if (result <= 0)
            break;
        for (;;) {
            ssize_t count = read(SHTTY, chunk, sizeof(chunk));
            if (count <= 0)
                break;
            if (response_length + (size_t)count > sizeof(response))
                count = (ssize_t)(sizeof(response) - response_length);
            memcpy(response + response_length, chunk, (size_t)count);
            response_length += (size_t)count;
            if (response_length == sizeof(response))
                break;
        }
        if (!terminal_colors.have_background)
            terminal_colors.have_background =
                parse_osc_color(response, response_length, 11, terminal_colors.background);
        if (!terminal_colors.have_cursor)
            terminal_colors.have_cursor =
                parse_osc_color(response, response_length, 12, terminal_colors.cursor);
        elapsed = (int)elapsed_milliseconds_since(&query_started);
    }
    flags = fcntl(SHTTY, F_GETFL, 0);
    if (flags >= 0)
        (void)fcntl(SHTTY, F_SETFL, original_flags);
}

void keel_reset_cursor_animation(void)
{
    (void)gettimeofday(&cursor_blink_started, NULL);
}

static void run_cursor_animation_child(int write_fd)
{
    unsigned char tick = 1;

    for (;;) {
        struct timespec interval = {
            .tv_sec = 0,
            .tv_nsec = KEEL_CURSOR_ANIMATION_INTERVAL_NS,
        };
        ssize_t written;

        while (nanosleep(&interval, &interval) < 0 && errno == EINTR) {
        }
        do {
            written = write(write_fd, &tick, sizeof(tick));
        } while (written < 0 && errno == EINTR);
        if (written != (ssize_t)sizeof(tick))
            _exit(0);
    }
}

int keel_start_cursor_animation(void)
{
    int pipes[2];
    int read_fd;
    int write_fd;
    int flags;
    pid_t pid;

    setiparam_no_convert("REPLY", -1);
    if (cursor_animation_fd >= 0 || SHTTY < 0)
        return cursor_animation_fd >= 0;
    if (pipe(pipes) < 0)
        return 0;
    read_fd = movefd(pipes[0]);
    write_fd = movefd(pipes[1]);
    if (read_fd < 0 || write_fd < 0) {
        if (read_fd >= 0)
            zclose(read_fd);
        if (write_fd >= 0)
            zclose(write_fd);
        return 0;
    }
    pid = fork();
    if (pid == 0) {
        close(read_fd);
        run_cursor_animation_child(write_fd);
    }
    if (pid < 0) {
        zclose(read_fd);
        zclose(write_fd);
        return 0;
    }
    zclose(write_fd);
    flags = fcntl(read_fd, F_GETFL, 0);
    if (flags >= 0)
        (void)fcntl(read_fd, F_SETFL, flags | O_NONBLOCK);
    addmodulefd(read_fd, KEEL_FDT_MODULE);
    cursor_animation_pid = pid;
    cursor_animation_fd = read_fd;
    setiparam_no_convert("REPLY", (long)read_fd);
    return 1;
}

void keel_stop_cursor_animation(void)
{
    if (cursor_animation_fd >= 0) {
        zclose(cursor_animation_fd);
        cursor_animation_fd = -1;
    }
    if (cursor_animation_pid > 0) {
        (void)kill(cursor_animation_pid, SIGKILL);
        while (waitpid(cursor_animation_pid, NULL, 0) < 0 && errno == EINTR) {
        }
        cursor_animation_pid = -1;
    }
}

void keel_cursor_animation_tick(void)
{
    unsigned char ticks[128];

    if (cursor_animation_fd < 0)
        return;
    for (;;) {
        ssize_t count = read(cursor_animation_fd, ticks, sizeof(ticks));
        if (count > 0)
            continue;
        if (count < 0 && errno == EINTR)
            continue;
        break;
    }
    keel_write_cursor_style(1);
}

static size_t copy_zle_line(void)
{
    mbstate_t state;
    size_t output_length = 0;
    int index;

    memset(&state, 0, sizeof(state));
    for (index = 0; zleline != NULL && index < zlell; index++) {
        char encoded[MB_LEN_MAX];
        size_t encoded_length = wcrtomb(encoded, zleline[index], &state);
        if (encoded_length == (size_t)-1) {
            memset(&state, 0, sizeof(state));
            encoded[0] = '?';
            encoded_length = 1;
        }
        if (output_length + encoded_length >= sizeof(line_buffer))
            break;
        memcpy(line_buffer + output_length, encoded, encoded_length);
        output_length += encoded_length;
    }
    line_buffer[output_length] = '\0';
    return output_length;
}

static size_t copy_cwd(void)
{
    if (getcwd(cwd_buffer, sizeof(cwd_buffer)) == NULL) {
        cwd_buffer[0] = '~';
        cwd_buffer[1] = '\0';
        return 1;
    }
    return strlen(cwd_buffer);
}

static void fill_host_snapshot(KeelNativeHostSnapshot *snapshot)
{
    copy_zle_line();
    copy_cwd();
    snapshot->abi_version = KEEL_NATIVE_ABI_VERSION;
    snapshot->buffer = (const unsigned char *)line_buffer;
    snapshot->buffer_len = strlen(line_buffer);
    snapshot->cursor_units = zlecs > 0 ? (size_t)zlecs : 0;
    snapshot->terminal_columns = (uint16_t)(zterm_columns > 0 ? zterm_columns : 1);
    snapshot->terminal_rows = (uint16_t)(zterm_lines > 0 ? zterm_lines : 1);
    snapshot->cursor_column = (uint16_t)(keel_zle_cursor_column > 0 ?
                                        keel_zle_cursor_column : 0);
    snapshot->cursor_row = (uint16_t)(keel_zle_cursor_line > 0 ?
                                     keel_zle_cursor_line : 0);
    snapshot->cwd = (const unsigned char *)cwd_buffer;
    snapshot->cwd_len = strlen(cwd_buffer);
    snapshot->keymap = (const unsigned char *)keymap_buffer;
    snapshot->keymap_len = strlen(keymap_buffer);
    snapshot->last_status = lastval;
    snapshot->redisplay_generation = keel_redisplay_generation;
}

void keel_observe_current_line(void)
{
    KeelNativeHostSnapshot snapshot;

    if (!runtime.active || SHTTY < 0)
        return;
    fill_host_snapshot(&snapshot);
    (void)keel_module_observe(&snapshot);
}

void keel_write_rust_payload(size_t length)
{
    if (length > 0 && length <= sizeof(patch_buffer) && SHTTY >= 0)
        (void)write_all(patch_buffer, length);
}

void keel_write_cursor_style(int block)
{
    static const unsigned char hide_cursor[] = "\033[?25l";
    static const unsigned char show_cursor[] = "\033[?25h";
    const unsigned char *sequence;
    size_t length;

    sequence = block ? hide_cursor : show_cursor;
    length = block ? sizeof(hide_cursor) - 1 : sizeof(show_cursor) - 1;

    if (SHTTY >= 0 && block) {
        sequence = hide_cursor;
        (void)write_all(sequence, length);
        cursor_style_active = 1;
    } else if (SHTTY >= 0 && !block) {
        restore_fake_cursor_cell();
        sequence = show_cursor;
        (void)write_all(sequence, length);
        if (terminal_colors.have_cursor)
            write_cursor_color(terminal_colors.cursor);
        cursor_style_active = 0;
    }
    if (SHTTY >= 0 && block)
        write_fake_cursor_cell();
}

void keel_before_redraw(void)
{
    size_t length;

    if (!runtime.active || runtime.in_callback || !zleactive || SHTTY < 0)
        return;
    runtime.in_callback = 1;
    restore_fake_cursor_cell();
    if (shout != NULL)
        fflush(shout);
    length = keel_module_before_redraw(patch_buffer, sizeof(patch_buffer));
    keel_write_rust_payload(length);
    runtime.in_callback = 0;
}

void keel_after_redraw(void)
{
    KeelNativeHostSnapshot snapshot;
    size_t length;

    if (!runtime.active || runtime.in_callback || !zleactive || SHTTY < 0)
        return;
    keel_reset_cursor_animation();
    capture_fake_cursor_text();
    fill_host_snapshot(&snapshot);

    runtime.in_callback = 1;
    length = keel_module_after_redraw(&snapshot, patch_buffer, sizeof(patch_buffer));
    keel_write_rust_payload(length);
    keel_write_cursor_style(1);
    runtime.in_callback = 0;
}

int keel_replace_zle_line_with_selection(void)
{
    mbstate_t state;
    const char *source;
    size_t replacement_length;
    size_t wide_length;
    size_t wide_capacity = sizeof(wide_line_buffer) / sizeof(*wide_line_buffer) - 1;

    replacement_length = keel_module_selected_replacement(
        (unsigned char *)line_buffer, sizeof(line_buffer) - 1);
    if (replacement_length == 0 || replacement_length >= sizeof(line_buffer) || zleline == NULL)
        return 1;
    line_buffer[replacement_length] = '\0';

    memset(&state, 0, sizeof(state));
    source = line_buffer;
    wide_length = mbsrtowcs(wide_line_buffer, &source, wide_capacity, &state);
    if (wide_length == (size_t)-1 || source != NULL || wide_length > INT_MAX)
        return 1;
    wide_line_buffer[wide_length] = L'\0';

    zleline = zrealloc(zleline, (wide_length + 1) * sizeof(*zleline));
    wmemcpy(zleline, wide_line_buffer, wide_length + 1);
    zlell = (int)wide_length;
    zlecs = zlell;
    keel_module_suppress_overlay_for_line(
        (const unsigned char *)line_buffer, replacement_length);
    return 0;
}
