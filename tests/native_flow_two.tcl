# Prefix-aware completion keeps the command prefix when it inserts an option.
send "keel-test b"
expect_popup
send [format "%c%c%c" 27 91 66]
expect {
    -re {1/1; [0-9]+\.[0-9]ms} {}
    timeout {
        puts stderr "single real completion did not become selectable"
        exit 1
    }
}
send "\t"
send "\r"
expect {
    -re {keel-test:beta} {}
    timeout {
        puts stderr "completion insertion did not preserve the command prefix"
        exit 1
    }
}

# Escape dismisses the surface without changing the host line; typing again
# changes the line and reopens the surface.
send "keel-test al"
expect_popup
send "\033"
after 200
send "x"
expect_popup
send "\003"
after 200

send "printf native-ok\r"
expect {
    -re {native-ok} {}
    timeout {
        puts stderr "accepted command did not execute normally"
        exit 1
    }
}
expect_native_prompt

# A resize still goes through the host redisplay path and the popup remains
# available after the terminal geometry changes.
send "stty rows 12 columns 48; kill -WINCH \$\$\r"
expect_native_prompt
send "keel-test al"
expect_popup
send "\003"
after 200
send "printf resize-ok\r"
expect {
    -re {resize-ok} {}
    timeout {
        puts stderr "resized session did not execute normally"
        exit 1
    }
}
expect_native_prompt

# Disable restores the host bindings; enable installs the Keel wrappers again.
send "keel disable\r"
expect_native_prompt
send "bindkey -M main '^I'\r"
expect {
    -re {expand-or-complete} {}
    timeout {
        puts stderr "disable did not restore the Tab binding"
        exit 1
    }
}
expect_native_prompt
send "keel enable\r"
expect_native_prompt
send "bindkey -M main '^I'\r"
expect {
    -re {_keel_accept_widget} {}
    timeout {
        puts stderr "enable did not install the Keel Tab binding"
        exit 1
    }
}
expect_native_prompt
send "keel status\r"
expect {
    -re {keel: enabled} {}
    timeout {
        puts stderr "keel status did not report an active session"
        exit 1
    }
}
expect_native_prompt
send "keel disable\r"
expect_native_prompt
send "printf disabled-ok\r"
expect {
    -re {disabled-ok} {}
    timeout {
        puts stderr "the shell was not usable after unloading Keel"
        exit 1
    }
}
expect_native_prompt

send "exit\r"
expect eof
file delete -force $startup_dir
