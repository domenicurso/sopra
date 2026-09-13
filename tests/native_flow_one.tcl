# Up/Down selects a Rust-owned item, and Tab replaces the ZLE line through
# the native widget rather than printing a second prompt.
send "keel-test al"
expect {
    -re {first result.*0/[0-9]+; [0-9]+ms} {}
    timeout {
        puts stderr "compadd descriptions were not captured"
        exit 1
    }
}
# Editing the same completion token reuses the broad provider result, so the
# Rust-side fuzzy filter is immediate instead of starting another zpty.
send [format "%c" 127]
expect {
    -re {[0-9]+/[0-9]+; 0ms} {}
    timeout {
        puts stderr "cached fuzzy filtering was not immediate"
        exit 1
    }
}
send [format "%c%c%c" 27 91 66]
expect {
    -re {1/[0-9]+; [0-9]+ms} {}
    timeout {
        puts stderr "selection movement did not redraw the selected item"
        exit 1
    }
}
send "\t"
send "\r"
expect {
    -re {keel-test:alpha} {}
    timeout {
        puts stderr "Tab did not insert the selected completion before execution"
        exit 1
    }
}
expect_native_prompt

# Ctrl-C clears the active line in place. Empty Enter only redisplays it, so
# the next real command still starts from the same native prompt.
send "\r"
after 200
send "printf empty-enter-ok\r"
expect {
    -re {empty-enter-ok} {}
    timeout {
        puts stderr "empty Enter left ZLE unable to execute a later command"
        exit 1
    }
}
expect_native_prompt

# Zsh's argument parser supplies flags, subcommands, and their descriptions;
# Keel must display those records instead of synthesizing them from help text.
send "keel-options --"
expect {
    -re {verbose} {}
    timeout {
        puts stderr "Zsh did not provide the verbose option"
        exit 1
    }
}
expect {
    -re {show verbose output} {}
    timeout {
        puts stderr "Zsh verbose option description was not captured"
        exit 1
    }
}
expect {
    -re {format} {}
    timeout {
        puts stderr "Zsh did not provide the format option"
        exit 1
    }
}
expect {
    -re {select output format.*0/[0-9]+; [0-9]+ms} {}
    timeout {
        puts stderr "Zsh option descriptions were not captured"
        exit 1
    }
}
send "\003"
after 200

# The completion set is captured from the real provider and fuzzy-ranked in
# Rust, so a non-contiguous query can find an option that prefix matching misses.
send "keel-fuzzy mchs"
expect {
    -re {print only matching files.*0/[0-9]+; [0-9]+ms} {}
    timeout {
        puts stderr "fuzzy completion did not find the non-prefix option"
        exit 1
    }
}
send "\003"
after 200

# More than twelve matches use a scrollable native viewport rather than
# growing into the rest of the terminal.
send "keel-many "
expect {
    -re {item-11.*0/20; [0-9]+ms} {}
    timeout {
        puts stderr "the first completion viewport did not contain twelve entries"
        exit 1
    }
}
for {set i 0} {$i < 11} {incr i} {
    send [format "%c%c%c" 27 91 66]
}
expect {
    -re {11/20; [0-9]+ms} {}
    timeout {
        puts stderr "long completion selection did not scroll the viewport"
        exit 1
    }
}
send "\003"
after 1000
expect_native_prompt
