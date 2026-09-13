#include "internal.h"

#include <errno.h>
#include <string.h>
#include <unistd.h>

#define KEEL_ZSH_META 0x83u

int keel_completion_read_worker(char **args)
{
    int reached_eof = 0;

    if ((args != NULL && args[0] != NULL) || keel_completion_worker_fd < 0)
        return 2;
    while (keel_completion_response_length < sizeof(keel_completion_response)) {
        ssize_t count = read(keel_completion_worker_fd,
                             keel_completion_response + keel_completion_response_length,
                             sizeof(keel_completion_response) -
                                 keel_completion_response_length);
        if (count > 0) {
            keel_completion_response_length += (size_t)count;
            if (memchr(keel_completion_response, 0x1d,
                       keel_completion_response_length) != NULL)
                break;
            continue;
        }
        if (count == 0)
            reached_eof = 1;
        if (count < 0 && errno == EINTR)
            continue;
        if (count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK))
            break;
        break;
    }
    if (memchr(keel_completion_response, 0x1d, keel_completion_response_length) == NULL)
        return reached_eof || keel_completion_response_length ==
                                  sizeof(keel_completion_response)
                   ? 2
                   : 1;
    setsparam("REPLY", metafy((char *)keel_completion_response,
                               (int)keel_completion_response_length, KEEL_META_DUP));
    return 0;
}

int keel_completion_stop_worker(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    keel_completion_close_worker(1);
    keel_completion_option_mode = 0;
    return 0;
}

int keel_completion_finish_capture(char **args)
{
    if (args == NULL || args[0] == NULL || args[1] == NULL || args[2] != NULL)
        return 1;
    keel_completion_capture_finish(args[0], args[1]);
    return 0;
}

void keel_completion_worker_shutdown(void)
{
    keel_completion_close_worker(1);
    keel_completion_option_mode = 0;
}
