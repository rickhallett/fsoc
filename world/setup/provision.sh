#!/bin/bash
# Provision the bandit game world at image build time.
#  - creates bandit0..$MAX_LEVEL as real users
#  - generates a fresh random password per user (never OverTheWire's own)
#  - stores it in /etc/bandit_pass/banditN (mode 0400, owned by banditN)
#  - sets each user's SSH login password to that same value
#  - plants each level's puzzle files with the correct ownership/perms
set -eu

MAX_LEVEL="${MAX_LEVEL:-11}"
PASSDIR=/etc/bandit_pass
mkdir -p "$PASSDIR"
chmod 755 "$PASSDIR"

genpass() { tr -dc 'A-Za-z0-9' </dev/urandom | head -c 32; }

# --- create users and passwords -------------------------------------------
for n in $(seq 0 "$MAX_LEVEL"); do
    u="bandit$n"
    useradd -m -s /bin/bash "$u"

    if [ "$n" -eq 0 ]; then
        pw="bandit0"          # the one published starting password
    else
        pw="$(genpass)"
    fi

    printf '%s\n' "$pw" > "$PASSDIR/$u"
    chown "$u:$u" "$PASSDIR/$u"
    chmod 400 "$PASSDIR/$u"

    echo "$u:$pw" | chpasswd
done

# home dirs: readable so the game feels like the real thing, but the
# password files planted inside are what matter.
for n in $(seq 0 "$MAX_LEVEL"); do
    chmod 755 "/home/bandit$n"
done

# password for level N (what banditN's login uses); helper for planting.
pw_of() { cat "$PASSDIR/bandit$1"; }

# --- plant the puzzles -----------------------------------------------------
# Each level file lives in bandit(N)'s home and contains bandit(N+1)'s
# password, which the player extracts using that level's technique.
/opt/wargame/setup/plant-levels.sh

echo "Provisioned bandit0..bandit$MAX_LEVEL"
