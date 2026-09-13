#include "internal.h"

#include <errno.h>
#include <sys/time.h>
#include <unistd.h>

int keel_write_all(const unsigned char *data, size_t length)
{
    while (length > 0) {
        ssize_t written = write(SHTTY, data, length);
        if (written < 0 && errno == EINTR)
            continue;
        if (written <= 0)
            return 0;
        data += written;
        length -= (size_t)written;
    }
    return 1;
}

unsigned long long keel_elapsed_milliseconds_since(const struct timeval *started)
{
    struct timeval now;
    long long seconds;
    long microseconds;

    (void)gettimeofday(&now, NULL);
    seconds = (long long)now.tv_sec - (long long)started->tv_sec;
    microseconds = now.tv_usec - started->tv_usec;
    if (microseconds < 0) {
        seconds--;
        microseconds += 1000000;
    }
    if (seconds < 0)
        return 0;
    return (unsigned long long)seconds * 1000u +
           (unsigned long long)microseconds / 1000u;
}
