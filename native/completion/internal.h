#ifndef KEEL_COMPLETION_INTERNAL_H
#define KEEL_COMPLETION_INTERNAL_H

#include "../keel_zsh_module_internal.h"

#include <sys/types.h>

extern Widget completion_worker_start_widget;
extern Widget completion_worker_read_widget;
extern Widget completion_worker_stop_widget;
extern Widget completion_capture_finish_widget;
extern pid_t keel_completion_worker_pid;
extern int keel_completion_worker_fd;
extern unsigned char keel_completion_response[KEEL_MAX_HOST_BYTES + 128u];
extern size_t keel_completion_response_length;

int keel_completion_start_worker(char **args);
int keel_completion_read_worker(char **args);
int keel_completion_stop_worker(char **args);
int keel_completion_finish_capture(char **args);
void keel_completion_close_worker(int terminate);

#endif
