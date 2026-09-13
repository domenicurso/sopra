#include "internal.h"

int keel_widget_start_cursor_animation(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    return keel_start_cursor_animation() ? 0 : 1;
}

int keel_widget_read_cursor_animation(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    keel_cursor_animation_tick();
    return 0;
}

int keel_widget_stop_cursor_animation(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    keel_stop_cursor_animation();
    return 0;
}
