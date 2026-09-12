#include "zsh_abi.h"

#include <errno.h>
#include <limits.h>
#include <locale.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

extern void keel_module_init(void);
extern void keel_module_shutdown(void);
extern void keel_module_reset(void);
extern size_t keel_module_before_redraw(unsigned char *output, size_t capacity);
extern size_t keel_module_after_redraw(const void *snapshot, unsigned char *output,
                                       size_t capacity);
extern size_t keel_module_line_finish(unsigned char *output, size_t capacity);

typedef struct {
    uint32_t abi_version;
    const unsigned char *buffer;
    size_t buffer_len;
    size_t cursor_units;
    uint16_t terminal_columns;
    uint16_t terminal_rows;
    uint16_t cursor_column;
    uint16_t cursor_row;
    const unsigned char *cwd;
    size_t cwd_len;
    const unsigned char *keymap;
    size_t keymap_len;
    int32_t last_status;
    uint64_t redisplay_generation;
} KeelNativeHostSnapshot;

typedef struct {
    int active;
    int in_callback;
} KeelRuntime;

static Widget line_init_widget;
static Widget line_finish_widget;
static KeelRuntime runtime;
static unsigned char patch_buffer[1024 * 1024];
static char line_buffer[256 * 1024];
static char cwd_buffer[4096];
static const char keymap_buffer[] = "main";

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

static void write_rust_payload(size_t length)
{
    if (length > 0 && length <= sizeof(patch_buffer) && SHTTY >= 0)
        (void)write_all(patch_buffer, length);
}

static void keel_before_redraw(void)
{
    size_t length;

    if (!runtime.active || runtime.in_callback || !zleactive || SHTTY < 0)
        return;
    runtime.in_callback = 1;
    if (shout != NULL)
        fflush(shout);
    length = keel_module_before_redraw(patch_buffer, sizeof(patch_buffer));
    write_rust_payload(length);
    runtime.in_callback = 0;
}

static void keel_after_redraw(void)
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
    snapshot.keymap_len = sizeof(keymap_buffer) - 1;
    snapshot.last_status = lastval;
    snapshot.redisplay_generation = keel_redisplay_generation;

    runtime.in_callback = 1;
    length = keel_module_after_redraw(&snapshot, patch_buffer, sizeof(patch_buffer));
    write_rust_payload(length);
    runtime.in_callback = 0;
}

static int keel_line_init(char **args)
{
    (void)args;
    keel_module_init();
    return 0;
}

static int keel_line_finish(char **args)
{
    size_t length;

    (void)args;
    if (!runtime.active || SHTTY < 0)
        return 0;
    runtime.in_callback = 1;
    length = keel_module_line_finish(patch_buffer, sizeof(patch_buffer));
    write_rust_payload(length);
    runtime.in_callback = 0;
    return 0;
}

int setup_(Module module)
{
    (void)module;
    return 0;
}

int features_(Module module, char ***features)
{
    (void)module;
    (void)features;
    return 1;
}

int enables_(Module module, int **enables)
{
    (void)module;
    (void)enables;
    return 1;
}

int boot_(Module module)
{
    (void)module;
    if (keel_zsh_abi_version != KEEL_ZSH_ABI_VERSION)
        return 1;
    (void)setlocale(LC_CTYPE, "");
    line_init_widget = addzlefunction("keel-native-line-init", keel_line_init,
                                      KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    if (line_init_widget == NULL)
        return 1;
    line_finish_widget = addzlefunction("keel-native-line-finish", keel_line_finish,
                                        KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    if (line_finish_widget == NULL) {
        deletezlefunction(line_init_widget);
        line_init_widget = NULL;
        return 1;
    }
    runtime.active = 1;
    runtime.in_callback = 0;
    keel_pre_redraw_callback = keel_before_redraw;
    keel_post_redraw_callback = keel_after_redraw;
    keel_module_init();
    return 0;
}

int cleanup_(Module module)
{
    size_t length;

    (void)module;
    if (runtime.active && SHTTY >= 0) {
        runtime.in_callback = 1;
        length = keel_module_line_finish(patch_buffer, sizeof(patch_buffer));
        write_rust_payload(length);
        runtime.in_callback = 0;
    }
    runtime.active = 0;
    keel_pre_redraw_callback = NULL;
    keel_post_redraw_callback = NULL;
    keel_module_shutdown();
    if (line_finish_widget != NULL) {
        deletezlefunction(line_finish_widget);
        line_finish_widget = NULL;
    }
    if (line_init_widget != NULL) {
        deletezlefunction(line_init_widget);
        line_init_widget = NULL;
    }
    return 0;
}

int finish_(Module module)
{
    (void)module;
    return 0;
}
