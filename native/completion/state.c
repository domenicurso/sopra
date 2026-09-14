#include "internal.h"

pid_t keel_completion_worker_pid = -1;
int keel_completion_worker_fd = -1;
unsigned char keel_completion_response[KEEL_MAX_HOST_BYTES + 128u];
size_t keel_completion_response_length;
int keel_completion_path_mode;
