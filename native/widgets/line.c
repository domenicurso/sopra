#include "internal.h"

#include <errno.h>
#include <stdlib.h>
#include <string.h>

int keel_widget_clear_line(char **args)
{
    size_t length;

    (void)args;
    keel_stop_cursor_animation();
    keel_completion_worker_shutdown();
    keel_clear_fake_cursor_cell();
    if (zleline != NULL) {
        zleline[0] = L'\0';
        zlell = 0;
        zlecs = 0;
    }
    if (runtime.active && SHTTY >= 0) {
        runtime.in_callback = 1;
        length = keel_module_line_finish(patch_buffer, sizeof(patch_buffer));
        keel_write_rust_payload(length);
        keel_write_cursor_style(0);
        runtime.in_callback = 0;
    }
    return 0;
}

int keel_widget_set_suggestions(char **args)
{
    char *end;
    const char *payload;
    unsigned long long completion_tenths_ms = 0;
    size_t length;

    if (args == NULL || args[0] == NULL || args[0][0] != 'K')
        return 1;
    payload = args[0] + 1;
    if (args[1] != NULL) {
        errno = 0;
        completion_tenths_ms = strtoull(args[1], &end, 10);
        if (errno != 0 || end == args[1] || *end != '\0')
            return 1;
    }
    if (args[2] != NULL)
        return 1;
    length = strlen(payload);
    if (length > KEEL_MAX_HOST_BYTES)
        return 1;
    keel_observe_current_line();
    return keel_module_set_suggestions((const unsigned char *)payload, length,
                                       (uint64_t)completion_tenths_ms) ? 0 : 1;
}

int keel_widget_refresh_suggestions(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    return keel_module_refresh_suggestions(0) ? 0 : 1;
}
