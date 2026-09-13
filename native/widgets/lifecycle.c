#include "internal.h"

int keel_widget_line_init(char **args)
{
    (void)args;
    keel_module_init();
    keel_set_zle_line_origin();
    runtime.line_active = 1;
    keel_reset_cursor_animation();
    return 0;
}

int keel_widget_line_finish(char **args)
{
    size_t length;

    (void)args;
    runtime.line_active = 0;
    if (!runtime.active)
        return 0;
    if (SHTTY < 0)
        return 0;
    keel_stop_cursor_animation();
    keel_clear_fake_cursor_cell();
    runtime.in_callback = 1;
    length = keel_module_line_finish(patch_buffer, sizeof(patch_buffer));
    keel_write_rust_payload(length);
    keel_write_cursor_style(0);
    runtime.in_callback = 0;
    return 0;
}
