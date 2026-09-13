#include "internal.h"

int keel_widget_select_previous(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    (void)keel_module_move_selection(-1);
    return 0;
}

int keel_widget_select_next(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    (void)keel_module_move_selection(1);
    return 0;
}

int keel_widget_accept_selection(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    return keel_replace_zle_line_with_selection();
}

int keel_widget_dismiss_overlay(char **args)
{
    (void)args;
    if (!keel_module_has_suggestions())
        return 1;
    (void)keel_module_dismiss_overlay();
    return 0;
}
