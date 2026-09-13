# Standard completion functions must keep their option descriptions through
# the native Cmatch capture path.
send "grep --ign"
expect {
    -re {case-insensitive} {}
    timeout {
        puts stderr "standard Zsh option descriptions were not captured"
        exit 1
    }
}
expect {
    -re {(?:[0-9]+)?/[0-9]+; [0-9]+\.[0-9]ms} {}
    timeout {
        puts stderr "standard Zsh options did not render"
        exit 1
    }
}
send "\003"
expect_native_prompt

# Generated providers expose their normal Zsh completion function lazily, so
# command-owned descriptions and options use the same native capture path.
send "keel-generated-cli "
expect {
    -re {run command} {}
    timeout {
        puts stderr "generated Zsh provider was not loaded"
        exit 1
    }
}
expect {
    -re {(?:[0-9]+)?/[0-9]+; [0-9]+\.[0-9]ms} {}
    timeout {
        puts stderr "generated provider popup did not finish rendering"
        exit 1
    }
}
send "\003"
expect_native_prompt
send "keel-generated-cli --"
expect {
    -re {verbose mode} {}
    timeout {
        puts stderr "generated provider options were not captured"
        exit 1
    }
}
expect {
    -re {(?:[0-9]+)?/[0-9]+; [0-9]+\.[0-9]ms} {}
    timeout {
        puts stderr "generated option popup did not finish rendering"
        exit 1
    }
}
send "\003"
expect_native_prompt

# The bundled _git provider supplies real subcommands and descriptions.
send "git ch"
expect {
    -re {checkout branch or paths to working tree} {}
    timeout {
        puts stderr "standard Zsh subcommands were not captured"
        exit 1
    }
}
send "\003"
expect_native_prompt

# Tab accepts an unselected sole result without changing the multi-result
# rule that Down first selects item zero.
send "keel-test bet"
expect {
    -re {third result} {}
    timeout {
        puts stderr "sole unselected completion did not render"
        exit 1
    }
}
expect {
    -re {(?:[0-9]+)?/1; [0-9]+\.[0-9]ms} {}
    timeout {
        puts stderr "sole completion popup did not finish rendering"
        exit 1
    }
}
send "\t"
send "\r"
expect {
    -re {keel-test:beta} {}
    timeout {
        puts stderr "Tab did not accept the sole unselected result"
        exit 1
    }
}
expect_native_prompt

# Refining a selected result resets selection to the first item in the new
# ranked set instead of retaining the old numeric index.
send "keel-test a"
expect {
    -re {(?:[0-9]+)?/3; [0-9]+\.[0-9]ms} {}
    timeout {
        puts stderr "selection refinement setup did not render"
        exit 1
    }
}
send [format "%c%c%c" 27 91 66]
send [format "%c%c%c" 27 91 66]
expect {
    -re {2/3; [0-9]+\.[0-9]ms} {}
    timeout {
        puts stderr "second result was not selected before refinement"
        exit 1
    }
}
send "l"
expect {
    -re {first result} {}
    timeout {
        puts stderr "refining a selected query did not select the first result"
        exit 1
    }
}
expect {
    -re {1/2; 0\.0ms} {}
    timeout {
        puts stderr "refined completion popup did not finish rendering"
        exit 1
    }
}
send "\t"
send "\r"
expect {
    -re {keel-test:alpha} {}
    timeout {
        puts stderr "the refined first result was not accepted"
        exit 1
    }
}
expect_native_prompt
