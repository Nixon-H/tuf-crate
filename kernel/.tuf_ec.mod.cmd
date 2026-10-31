savedcmd_tuf_ec.mod := printf '%s\n'   tuf_ec.o | awk '!x[$$0]++ { print("./"$$0) }' > tuf_ec.mod
