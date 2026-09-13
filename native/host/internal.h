#ifndef KEEL_HOST_INTERNAL_H
#define KEEL_HOST_INTERNAL_H

#include "../keel_zsh_module_internal.h"

#include <sys/time.h>

int keel_write_all(const unsigned char *data, size_t length);
unsigned long long keel_elapsed_milliseconds_since(const struct timeval *started);

#endif
