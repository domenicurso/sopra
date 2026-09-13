#include "keel_zsh_module_internal.h"

#include <errno.h>
#include <stdlib.h>
#include <string.h>

static int keel_line_init(char **args)
{
    (void)args;
    keel_module_init();
    keel_query_terminal_colors();
    keel_reset_cursor_animation();
    return 0;
}

static int keel_line_finish(char **args)
{
    size_t length;

    (void)args;
    if (!runtime.active)
        return 0;
    keel_stop_cursor_animation();
    if (SHTTY < 0)
        return 0;
    runtime.in_callback = 1;
    length = keel_module_line_finish(patch_buffer, sizeof(patch_buffer));
    keel_write_rust_payload(length);
    keel_write_cursor_style(0);
    runtime.in_callback = 0;
    return 0;
}

static int keel_start_cursor(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    return keel_start_cursor_animation() ? 0 : 1;
}

static int keel_read_cursor(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    keel_cursor_animation_tick();
    return 0;
}

static int keel_stop_cursor(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    keel_stop_cursor_animation();
    return 0;
}

static Widget cursor_start_widget;
static Widget cursor_read_widget;
static Widget cursor_stop_widget;

static int keel_select_previous(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    (void)keel_module_move_selection(-1);
    return 0;
}

static int keel_select_next(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    (void)keel_module_move_selection(1);
    return 0;
}

static int keel_accept_selection(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    return keel_replace_zle_line_with_selection();
}

static int keel_dismiss_overlay(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    (void)keel_module_dismiss_overlay();
    return 0;
}

static int keel_clear_line(char **args)
{
    size_t length;

    (void)args;
    keel_completion_worker_shutdown();
    if (zleline != NULL) {
        zleline[0] = L'\0';
        zlell = 0;
        zlecs = 0;
    }
    if (runtime.active && SHTTY >= 0) {
        runtime.in_callback = 1;
        length = keel_module_line_finish(patch_buffer, sizeof(patch_buffer));
        keel_write_rust_payload(length);
        runtime.in_callback = 0;
    }
    return 0;
}

static int keel_set_suggestions(char **args)
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

static int keel_refresh_suggestions(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    return keel_module_refresh_suggestions(0) ? 0 : 1;
}

void keel_delete_widgets(void)
{
    keel_delete_completion_widgets();
    if (dismiss_widget != NULL) {
        deletezlefunction(dismiss_widget);
        dismiss_widget = NULL;
    }
    if (accept_widget != NULL) {
        deletezlefunction(accept_widget);
        accept_widget = NULL;
    }
    if (select_next_widget != NULL) {
        deletezlefunction(select_next_widget);
        select_next_widget = NULL;
    }
    if (select_previous_widget != NULL) {
        deletezlefunction(select_previous_widget);
        select_previous_widget = NULL;
    }
    if (clear_line_widget != NULL) {
        deletezlefunction(clear_line_widget);
        clear_line_widget = NULL;
    }
    if (set_suggestions_widget != NULL) {
        deletezlefunction(set_suggestions_widget);
        set_suggestions_widget = NULL;
    }
    if (refresh_suggestions_widget != NULL) {
        deletezlefunction(refresh_suggestions_widget);
        refresh_suggestions_widget = NULL;
    }
    if (cursor_stop_widget != NULL) {
        deletezlefunction(cursor_stop_widget);
        cursor_stop_widget = NULL;
    }
    if (cursor_read_widget != NULL) {
        deletezlefunction(cursor_read_widget);
        cursor_read_widget = NULL;
    }
    if (cursor_start_widget != NULL) {
        deletezlefunction(cursor_start_widget);
        cursor_start_widget = NULL;
    }
    if (line_finish_widget != NULL) {
        deletezlefunction(line_finish_widget);
        line_finish_widget = NULL;
    }
    if (line_init_widget != NULL) {
        deletezlefunction(line_init_widget);
        line_init_widget = NULL;
    }
}

int keel_register_widgets(void)
{
    line_init_widget = addzlefunction("keel-native-line-init", keel_line_init,
                                      KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    line_finish_widget = addzlefunction("keel-native-line-finish", keel_line_finish,
                                        KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    select_previous_widget = addzlefunction("keel-native-select-previous",
                                            keel_select_previous,
                                            KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    select_next_widget = addzlefunction("keel-native-select-next", keel_select_next,
                                        KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    accept_widget = addzlefunction("keel-native-accept-selection", keel_accept_selection,
                                   KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    dismiss_widget = addzlefunction("keel-native-dismiss-overlay", keel_dismiss_overlay,
                                    KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    clear_line_widget = addzlefunction("keel-native-clear-line", keel_clear_line,
                                       KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    set_suggestions_widget = addzlefunction("keel-native-set-suggestions",
                                            keel_set_suggestions,
                                            KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    refresh_suggestions_widget = addzlefunction("keel-native-refresh-suggestions",
                                                keel_refresh_suggestions,
                                                KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    cursor_start_widget = addzlefunction("keel-native-start-cursor-animation",
                                         keel_start_cursor,
                                         KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    cursor_read_widget = addzlefunction("keel-native-read-cursor-animation",
                                        keel_read_cursor,
                                        KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    cursor_stop_widget = addzlefunction("keel-native-stop-cursor-animation",
                                        keel_stop_cursor,
                                        KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST);
    if (!keel_register_completion_widgets()) {
        keel_delete_widgets();
        return 0;
    }
    if (line_init_widget == NULL || line_finish_widget == NULL ||
        select_previous_widget == NULL || select_next_widget == NULL ||
        accept_widget == NULL || dismiss_widget == NULL || clear_line_widget == NULL ||
        set_suggestions_widget == NULL || refresh_suggestions_widget == NULL ||
        cursor_start_widget == NULL || cursor_read_widget == NULL ||
        cursor_stop_widget == NULL) {
        keel_delete_widgets();
        return 0;
    }
    return 1;
}
