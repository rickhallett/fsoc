#!/bin/bash
set -e
ssh-keygen -A >/dev/null 2>&1 || true
echo "machine up: $(hostname)"
exec /usr/sbin/sshd -D -e
