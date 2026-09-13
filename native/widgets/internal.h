#ifndef KEEL_WIDGETS_INTERNAL_H
#define KEEL_WIDGETS_INTERNAL_H

#include "../keel_zsh_module_internal.h"

int keel_widget_line_init(char **args);
int keel_widget_line_finish(char **args);
int keel_widget_select_previous(char **args);
int keel_widget_select_next(char **args);
int keel_widget_accept_selection(char **args);
int keel_widget_dismiss_overlay(char **args);
int keel_widget_clear_line(char **args);
int keel_widget_set_suggestions(char **args);
int keel_widget_refresh_suggestions(char **args);
int keel_widget_start_cursor_animation(char **args);
int keel_widget_read_cursor_animation(char **args);
int keel_widget_stop_cursor_animation(char **args);

#endif
