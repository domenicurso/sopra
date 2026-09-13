#include "internal.h"

#include "../cursor/internal.h"

#include <limits.h>
#include <locale.h>
#include <string.h>
#include <unistd.h>

static int have_last_cursor_position;
static uint16_t last_cursor_column;
static size_t last_cursor_units;
static int last_cursor_line;
static int have_zle_line_origin;
static uint16_t zle_line_origin;

void keel_set_zle_line_origin(void)
{
    int terminal_row = keel_capture_zle_line_origin();

    have_zle_line_origin = terminal_row > 0;
    if (have_zle_line_origin) {
        zle_line_origin = (uint16_t)(terminal_row - 1);
    }
}

void keel_adjust_zle_line_origin(unsigned int rows)
{
    if (!have_zle_line_origin || rows == 0)
        return;
    zle_line_origin = rows >= zle_line_origin ? 0 :
                                                   (uint16_t)(zle_line_origin - rows);
}

static uint16_t physical_cursor_row(void)
{
    uint32_t row;

    if (!have_zle_line_origin || keel_zle_cursor_line <= 0)
        return have_zle_line_origin ? zle_line_origin :
                                      (uint16_t)(keel_zle_cursor_line > 0 ?
                                                     keel_zle_cursor_line : 0);
    row = (uint32_t)zle_line_origin + (uint32_t)keel_zle_cursor_line;
    if (zterm_lines > 0 && row >= (uint32_t)zterm_lines)
        row = (uint32_t)zterm_lines - 1;
    return row > UINT16_MAX ? UINT16_MAX : (uint16_t)row;
}

static void reset_animation_if_cursor_moved(const KeelNativeHostSnapshot *snapshot)
{
    if (!have_last_cursor_position || last_cursor_column != snapshot->cursor_column ||
        last_cursor_units != snapshot->cursor_units || last_cursor_line != keel_zle_cursor_line) {
        keel_reset_cursor_animation();
    }
    last_cursor_column = snapshot->cursor_column;
    last_cursor_units = snapshot->cursor_units;
    last_cursor_line = keel_zle_cursor_line;
    have_last_cursor_position = 1;
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
    snapshot->cursor_row = physical_cursor_row();
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
    static const unsigned char show_cursor[] = "\033[?25h";
    size_t visible_length = length;

    if (length == 0 || length > sizeof(patch_buffer) || SHTTY < 0)
        return;
    /* The native fake cursor owns visibility for the entire active ZLE line. */
    if (visible_length >= sizeof(show_cursor) - 1 &&
        memcmp(patch_buffer + visible_length - (sizeof(show_cursor) - 1), show_cursor,
               sizeof(show_cursor) - 1) == 0)
        visible_length -= sizeof(show_cursor) - 1;
    if (visible_length > 0)
        (void)keel_write_all(patch_buffer, visible_length);
}

void keel_write_cursor_style(int block)
{
    static const unsigned char hide_cursor[] = "\033[?25l";
    static const unsigned char show_cursor[] = "\033[?25h";
    const unsigned char *sequence = block ? hide_cursor : show_cursor;
    size_t length = block ? sizeof(hide_cursor) - 1 : sizeof(show_cursor) - 1;

    if (SHTTY < 0)
        return;
    if (block && !runtime.line_active)
        return;
    if (block) {
        if (keel_fake_cursor_cell_active) {
            (void)keel_write_all(sequence, length);
            keel_refresh_fake_cursor_cell();
        } else {
            (void)keel_write_all(sequence, length);
            keel_write_fake_cursor_cell();
        }
    } else {
        keel_restore_fake_cursor_cell();
        (void)keel_write_all(sequence, length);
        keel_restore_terminal_cursor_color();
    }
}

void keel_before_redraw(void)
{
    size_t length;

    if (!runtime.active || !runtime.line_active || runtime.in_callback || !zleactive || SHTTY < 0)
        return;
    runtime.in_callback = 1;
    keel_restore_fake_cursor_cell();
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
    uint16_t scroll_rows;

    if (!runtime.active || !runtime.line_active || runtime.in_callback || !zleactive || SHTTY < 0)
        return;
    keel_capture_fake_cursor_text();
    fill_host_snapshot(&snapshot);
    reset_animation_if_cursor_moved(&snapshot);

    runtime.in_callback = 1;
    length = keel_module_after_redraw(&snapshot, patch_buffer, sizeof(patch_buffer));
    scroll_rows = keel_module_last_scroll_rows();
    keel_adjust_zle_line_origin(scroll_rows);
    keel_write_rust_payload(length);
    keel_write_cursor_style(1);
    runtime.in_callback = 0;
}
