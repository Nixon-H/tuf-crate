savedcmd_tuf_rgb.mod := printf '%s\n'   tuf_rgb.o | awk '!x[$$0]++ { print("./"$$0) }' > tuf_rgb.mod
