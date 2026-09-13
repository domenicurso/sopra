#include "internal.h"

#include "../cursor/internal.h"

#include <limits.h>
#include <locale.h>
#include <string.h>
#include <unistd.h>

static int have_last_cursor_position;
static uint16_t last_cursor_column;
static uint16_t last_cursor_row;

static void reset_animation_if_cursor_moved(const KeelNativeHostSnapshot *snapshot)
{
    if (!have_last_cursor_position || last_cursor_column != snapshot->cursor_column ||
        last_cursor_row != snapshot->cursor_row) {
        keel_reset_cursor_animation();
    }
    last_cursor_column = snapshot->cursor_column;
    last_cursor_row = snapshot->cursor_row;
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
        (void)keel_write_all(patch_buffer, length);
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
    keel_restore_fake_cursor_cell();
    if (block) {
        (void)keel_write_all(sequence, length);
        keel_write_fake_cursor_cell();
    } else {
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

    if (!runtime.active || !runtime.line_active || runtime.in_callback || !zleactive || SHTTY < 0)
        return;
    keel_capture_fake_cursor_text();
    fill_host_snapshot(&snapshot);
    reset_animation_if_cursor_moved(&snapshot);

    runtime.in_callback = 1;
    length = keel_module_after_redraw(&snapshot, patch_buffer, sizeof(patch_buffer));
    keel_write_rust_payload(length);
    keel_write_cursor_style(1);
    runtime.in_callback = 0;
}
