#include "internal.h"

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <sys/time.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

enum {
    KEEL_CURSOR_BLINK_PERIOD_MS = 1800,
    KEEL_CURSOR_ACTIVITY_HOLD_MS = 300,
    KEEL_CURSOR_ANIMATION_INTERVAL_NS = 16 * 1000 * 1000,
};

static struct timeval cursor_blink_started;
static pid_t cursor_animation_pid = -1;
static int cursor_animation_fd = -1;

static double smoothstep(double value)
{
    if (value <= 0.0)
        return 0.0;
    if (value >= 1.0)
        return 1.0;
    return value * value * (3.0 - 2.0 * value);
}

double keel_cursor_opacity(void)
{
    unsigned long long elapsed = keel_elapsed_milliseconds_since(&cursor_blink_started);
    double visibility;

    if (elapsed < KEEL_CURSOR_ACTIVITY_HOLD_MS)
        return 1.0;
    elapsed -= KEEL_CURSOR_ACTIVITY_HOLD_MS;
    double phase = (double)(elapsed % KEEL_CURSOR_BLINK_PERIOD_MS) /
                   (double)KEEL_CURSOR_BLINK_PERIOD_MS;
    double half_cycle = phase * 2.0;

    if (half_cycle <= 1.0)
        visibility = 1.0 - smoothstep(half_cycle);
    else
        visibility = smoothstep(half_cycle - 1.0);
    return 0.4 + visibility * 0.6;
}

void keel_reset_cursor_animation(void)
{
    (void)gettimeofday(&cursor_blink_started, NULL);
}

static void run_cursor_animation_child(int write_fd)
{
    unsigned char tick = 1;
    const struct timespec interval = {
        .tv_sec = 0,
        .tv_nsec = KEEL_CURSOR_ANIMATION_INTERVAL_NS,
    };

    for (;;) {
        while (nanosleep(&interval, NULL) < 0 && errno == EINTR) {
        }
        if (write(write_fd, &tick, sizeof(tick)) != (ssize_t)sizeof(tick))
            _exit(0);
    }
}

int keel_start_cursor_animation(void)
{
    int pipes[2];
    int read_fd;
    int write_fd;
    int flags;
    pid_t pid;

    setiparam_no_convert("REPLY", -1);
    if (cursor_animation_fd >= 0 || SHTTY < 0)
        return cursor_animation_fd >= 0;
    if (pipe(pipes) < 0)
        return 0;
    read_fd = movefd(pipes[0]);
    write_fd = movefd(pipes[1]);
    if (read_fd < 0 || write_fd < 0) {
        if (read_fd >= 0)
            zclose(read_fd);
        if (write_fd >= 0)
            zclose(write_fd);
        return 0;
    }
    pid = fork();
    if (pid == 0) {
        close(read_fd);
        run_cursor_animation_child(write_fd);
    }
    if (pid < 0) {
        zclose(read_fd);
        zclose(write_fd);
        return 0;
    }
    zclose(write_fd);
    flags = fcntl(read_fd, F_GETFL, 0);
    if (flags >= 0)
        (void)fcntl(read_fd, F_SETFL, flags | O_NONBLOCK);
    addmodulefd(read_fd, KEEL_FDT_MODULE);
    cursor_animation_pid = pid;
    cursor_animation_fd = read_fd;
    setiparam_no_convert("REPLY", (long)read_fd);
    return 1;
}

void keel_stop_cursor_animation(void)
{
    if (cursor_animation_fd >= 0) {
        zclose(cursor_animation_fd);
        cursor_animation_fd = -1;
    }
    if (cursor_animation_pid > 0) {
        (void)kill(cursor_animation_pid, SIGKILL);
        while (waitpid(cursor_animation_pid, NULL, 0) < 0 && errno == EINTR) {
        }
        cursor_animation_pid = -1;
    }
}

void keel_cursor_animation_tick(void)
{
    unsigned char ticks[128];
    ssize_t count;

    if (cursor_animation_fd < 0)
        return;
    for (;;) {
        do {
            count = read(cursor_animation_fd, ticks, sizeof(ticks));
        } while (count < 0 && errno == EINTR);
        if (count > 0)
            continue;
        break;
    }
    if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK)
        return;
    keel_write_cursor_style(1);
}
