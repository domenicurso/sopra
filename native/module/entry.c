#include "../keel_zsh_module_internal.h"

#include <locale.h>

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
    if (!keel_register_widgets())
        return 1;
    runtime.active = 1;
    runtime.in_callback = 0;
    runtime.line_active = 0;
    keel_reset_cursor_animation();
    keel_pre_redraw_callback = keel_before_redraw;
    keel_post_redraw_callback = keel_after_redraw;
    keel_completion_match_callback = keel_completion_capture_match;
    keel_query_terminal_colors();
    keel_module_init();
    return 0;
}

int cleanup_(Module module)
{
    size_t length;

    (void)module;
    keel_stop_cursor_animation();
    runtime.line_active = 0;
    if (SHTTY >= 0)
        keel_clear_fake_cursor_cell();
    if (runtime.active && SHTTY >= 0) {
        runtime.in_callback = 1;
        length = keel_module_line_finish(patch_buffer, sizeof(patch_buffer));
        keel_write_rust_payload(length);
        runtime.in_callback = 0;
    }
    keel_write_cursor_style(0);
    runtime.active = 0;
    keel_pre_redraw_callback = NULL;
    keel_post_redraw_callback = NULL;
    keel_completion_match_callback = NULL;
    keel_completion_worker_shutdown();
    keel_module_shutdown();
    keel_delete_widgets();
    return 0;
}

int finish_(Module module)
{
    (void)module;
    keel_write_cursor_style(0);
    return 0;
}
