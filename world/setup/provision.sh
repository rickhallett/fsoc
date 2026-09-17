#!/bin/bash
# Provision the game world at image build time.
#  - creates ${PREFIX}0..$MAX_LEVEL as real users
#  - generates a fresh random password per user (never a published answer)
#  - stores it in $PASSDIR/${PREFIX}N (mode 0400, owned by that user)
#  - sets each user's SSH login password to that same value
#  - plants each level's puzzle files with the correct ownership/perms
set -eu

PREFIX="${USER_PREFIX:-bandit}"
PASSDIR="${PASS_DIR:-/etc/${PREFIX}_pass}"
MAX_LEVEL="${MAX_LEVEL:-11}"
export PREFIX PASSDIR MAX_LEVEL

mkdir -p "$PASSDIR"
chmod 755 "$PASSDIR"

genpass() { tr -dc 'A-Za-z0-9' </dev/urandom | head -c 32; }

# --- create users and passwords -------------------------------------------
for n in $(seq 0 "$MAX_LEVEL"); do
    u="${PREFIX}${n}"
    useradd -m -s /bin/bash "$u"

    if [ "$n" -eq 0 ]; then
        pw="${PREFIX}0"       # the one published starting password
    else
        pw="$(genpass)"
    fi

    printf '%s\n' "$pw" > "$PASSDIR/$u"
    chown "$u:$u" "$PASSDIR/$u"
    chmod 400 "$PASSDIR/$u"

    echo "$u:$pw" | chpasswd
done

for n in $(seq 0 "$MAX_LEVEL"); do
    chmod 755 "/home/${PREFIX}${n}"
done

# only the game users may log in over SSH
allow=""
for n in $(seq 0 "$MAX_LEVEL"); do allow="$allow ${PREFIX}${n}"; done
echo "AllowUsers${allow}" >> /etc/ssh/sshd_config

/opt/wargame/setup/plant-levels.sh

echo "Provisioned ${PREFIX}0..${PREFIX}${MAX_LEVEL} (pass dir: $PASSDIR)"
