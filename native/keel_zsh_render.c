#include "keel_zsh_module_internal.h"

#include <errno.h>
#include <limits.h>
#include <locale.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

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

void keel_write_rust_payload(size_t length)
{
    if (length > 0 && length <= sizeof(patch_buffer) && SHTTY >= 0)
        (void)write_all(patch_buffer, length);
}

void keel_write_cursor_style(int block)
{
    static const unsigned char block_cursor[] = "\033[2 q";
    static const unsigned char terminal_cursor[] = "\033[0 q";
    const unsigned char *sequence = block ? block_cursor : terminal_cursor;
    size_t length = block ? sizeof(block_cursor) - 1 : sizeof(terminal_cursor) - 1;

    if (SHTTY >= 0)
        (void)write_all(sequence, length);
}

void keel_before_redraw(void)
{
    size_t length;

    if (!runtime.active || runtime.in_callback || !zleactive || SHTTY < 0)
        return;
    runtime.in_callback = 1;
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
    copy_zle_line();
    copy_cwd();
    snapshot.abi_version = KEEL_NATIVE_ABI_VERSION;
    snapshot.buffer = (const unsigned char *)line_buffer;
    snapshot.buffer_len = strlen(line_buffer);
    snapshot.cursor_units = zlecs > 0 ? (size_t)zlecs : 0;
    snapshot.terminal_columns = (uint16_t)(zterm_columns > 0 ? zterm_columns : 1);
    snapshot.terminal_rows = (uint16_t)(zterm_lines > 0 ? zterm_lines : 1);
    snapshot.cursor_column = (uint16_t)(keel_zle_cursor_column > 0 ?
                                        keel_zle_cursor_column : 0);
    snapshot.cursor_row = (uint16_t)(keel_zle_cursor_line > 0 ?
                                     keel_zle_cursor_line : 0);
    snapshot.cwd = (const unsigned char *)cwd_buffer;
    snapshot.cwd_len = strlen(cwd_buffer);
    snapshot.keymap = (const unsigned char *)keymap_buffer;
    snapshot.keymap_len = strlen(keymap_buffer);
    snapshot.last_status = lastval;
    snapshot.redisplay_generation = keel_redisplay_generation;

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
