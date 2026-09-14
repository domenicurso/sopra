#include "internal.h"

Widget completion_worker_start_widget;
Widget completion_worker_read_widget;
Widget completion_worker_stop_widget;
Widget completion_capture_finish_widget;
Widget completion_path_mode_widget;

static void delete_widget(Widget *widget)
{
    if (*widget != NULL) {
        deletezlefunction(*widget);
        *widget = NULL;
    }
}

void keel_delete_completion_widgets(void)
{
    delete_widget(&completion_capture_finish_widget);
    delete_widget(&completion_path_mode_widget);
    delete_widget(&completion_worker_stop_widget);
    delete_widget(&completion_worker_read_widget);
    delete_widget(&completion_worker_start_widget);
}

int keel_register_completion_widgets(void)
{
    int flags = KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST;

    completion_worker_start_widget = addzlefunction(
        "keel-native-start-completion", keel_completion_start_worker, flags);
    completion_worker_read_widget = addzlefunction(
        "keel-native-read-completion", keel_completion_read_worker, flags);
    completion_worker_stop_widget = addzlefunction(
        "keel-native-stop-completion", keel_completion_stop_worker, flags);
    completion_capture_finish_widget = addzlefunction(
        "keel-native-finish-capture", keel_completion_finish_capture, flags);
    completion_path_mode_widget = addzlefunction(
        "keel-native-set-completion-path-mode", keel_completion_set_path_mode, flags);
    return completion_worker_start_widget != NULL &&
           completion_worker_read_widget != NULL && completion_worker_stop_widget != NULL &&
           completion_capture_finish_widget != NULL && completion_path_mode_widget != NULL;
}
