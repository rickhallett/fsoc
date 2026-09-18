#!/bin/bash
# Provision the operator account, a shared ops key (baked in the image, not a
# real secret), an ssh config for frictionless pivots, and the themed shell.
set -eu

useradd -m -s /bin/bash -g operator operator
install -d -o operator -g operator -m 700 /home/operator/.ssh

# One ops key shared across the fleet: every machine authorizes it, every
# machine carries the private half, so `ssh <host>` just works.
ssh-keygen -t ed25519 -N '' -C ops@fsoc -f /home/operator/.ssh/id_ed25519 >/dev/null
cp /home/operator/.ssh/id_ed25519.pub /home/operator/.ssh/authorized_keys

cat > /home/operator/.ssh/config <<CFG
Host *
  User operator
  StrictHostKeyChecking accept-new
  UserKnownHostsFile ~/.ssh/known_hosts
CFG

chown -R operator:operator /home/operator/.ssh
chmod 600 /home/operator/.ssh/id_ed25519 /home/operator/.ssh/authorized_keys /home/operator/.ssh/config
chmod 644 /home/operator/.ssh/id_ed25519.pub

cat /opt/machine/theme.bashrc >> /home/operator/.bashrc
echo "provisioned operator@$(hostname 2>/dev/null || echo machine)"
