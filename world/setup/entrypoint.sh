#!/bin/bash
# Foreground sshd so the container's lifetime == the game's lifetime.
set -e
# Host keys are generated at build for stable fingerprints across restarts;
# regenerate here only if missing (e.g. keys were volume-mounted away).
ssh-keygen -A >/dev/null 2>&1 || true
echo "Bandit game world up. SSH: ssh bandit0@localhost -p 2220 (password: bandit0)"
exec /usr/sbin/sshd -D -e
