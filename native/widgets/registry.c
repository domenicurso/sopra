#include "internal.h"

Widget line_init_widget;
Widget line_finish_widget;
Widget select_previous_widget;
Widget select_next_widget;
Widget accept_widget;
Widget dismiss_widget;
Widget clear_line_widget;
Widget set_suggestions_widget;
Widget refresh_suggestions_widget;
Widget cursor_start_widget;
Widget cursor_read_widget;
Widget cursor_stop_widget;

static void delete_widget(Widget *widget)
{
    if (*widget != NULL) {
        deletezlefunction(*widget);
        *widget = NULL;
    }
}

void keel_delete_widgets(void)
{
    keel_delete_completion_widgets();
    delete_widget(&dismiss_widget);
    delete_widget(&accept_widget);
    delete_widget(&select_next_widget);
    delete_widget(&select_previous_widget);
    delete_widget(&clear_line_widget);
    delete_widget(&set_suggestions_widget);
    delete_widget(&refresh_suggestions_widget);
    delete_widget(&cursor_stop_widget);
    delete_widget(&cursor_read_widget);
    delete_widget(&cursor_start_widget);
    delete_widget(&line_finish_widget);
    delete_widget(&line_init_widget);
}

int keel_register_widgets(void)
{
    int flags = KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST;

    line_init_widget = addzlefunction("keel-native-line-init", keel_widget_line_init, flags);
    line_finish_widget = addzlefunction("keel-native-line-finish", keel_widget_line_finish,
                                        flags);
    select_previous_widget = addzlefunction("keel-native-select-previous",
                                            keel_widget_select_previous, flags);
    select_next_widget = addzlefunction("keel-native-select-next", keel_widget_select_next,
                                        flags);
    accept_widget = addzlefunction("keel-native-accept-selection",
                                   keel_widget_accept_selection, flags);
    dismiss_widget = addzlefunction("keel-native-dismiss-overlay",
                                    keel_widget_dismiss_overlay, flags);
    clear_line_widget = addzlefunction("keel-native-clear-line", keel_widget_clear_line, flags);
    set_suggestions_widget = addzlefunction("keel-native-set-suggestions",
                                            keel_widget_set_suggestions, flags);
    refresh_suggestions_widget = addzlefunction("keel-native-refresh-suggestions",
                                                keel_widget_refresh_suggestions, flags);
    cursor_start_widget = addzlefunction("keel-native-start-cursor-animation",
                                         keel_widget_start_cursor_animation, flags);
    cursor_read_widget = addzlefunction("keel-native-read-cursor-animation",
                                        keel_widget_read_cursor_animation, flags);
    cursor_stop_widget = addzlefunction("keel-native-stop-cursor-animation",
                                        keel_widget_stop_cursor_animation, flags);
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
