#!/bin/bash
# Plant each level's puzzle files. Runs as root at build time.
# ${PREFIX}(N)'s home holds the password for ${PREFIX}(N+1), extractable
# only via that level's intended technique. Guarded by MAX_LEVEL.
#
# The mechanics (file sizes, ownership, the word "millionth", ports) are
# the invariant wargame; only the user names are skinned via $PREFIX.
set -eu

PREFIX="${PREFIX:-${USER_PREFIX:-bandit}}"
PASSDIR="${PASSDIR:-${PASS_DIR:-/etc/${PREFIX}_pass}}"
MAX_LEVEL="${MAX_LEVEL:-11}"

u() { printf '%s%s' "$PREFIX" "$1"; }              # user name for level n
home() { printf '/home/%s%s' "$PREFIX" "$1"; }     # home dir for level n
pw() { cat "$PASSDIR/$(u "$1")"; }                  # password of level n
have() { [ "$1" -le "$MAX_LEVEL" ]; }
noise() { tr -dc 'A-Za-z0-9' </dev/urandom | head -c "$1"; }

# ---- L0 -> 1 : readme ------------------------------------------------------
if have 1; then
    f="$(home 0)/readme"
    pw 1 > "$f"; chown "$(u 0):$(u 0)" "$f"; chmod 644 "$f"
fi

# ---- L1 -> 2 : a file literally named "-" ---------------------------------
if have 2; then
    f="$(home 1)/-"
    pw 2 > "$f"; chown "$(u 1):$(u 1)" "$f"; chmod 644 "$f"
fi

# ---- L2 -> 3 : spaces in the filename -------------------------------------
if have 3; then
    f="$(home 2)/--spaces in this filename--"
    pw 3 > "$f"; chown "$(u 2):$(u 2)" "$f"; chmod 644 "$f"
fi

# ---- L3 -> 4 : hidden file in inhere/ -------------------------------------
if have 4; then
    d="$(home 3)/inhere"; mkdir -p "$d"
    f="$d/...Hiding-From-You"
    pw 4 > "$f"
    chown -R "$(u 3):$(u 3)" "$d"; chmod 755 "$d"; chmod 644 "$f"
fi

# ---- L4 -> 5 : only human-readable file in inhere/ ------------------------
if have 5; then
    d="$(home 4)/inhere"; mkdir -p "$d"
    real=$(( RANDOM % 10 ))
    for i in $(seq 0 9); do
        f=$(printf '%s/-file%02d' "$d" "$i")
        if [ "$i" -eq "$real" ]; then
            pw 5 > "$f"
        else
            head -c 200 /dev/urandom > "$f"
        fi
        chmod 644 "$f"
    done
    chown -R "$(u 4):$(u 4)" "$d"; chmod 755 "$d"
fi

# ---- L5 -> 6 : human-readable, 1033 bytes, not executable -----------------
if have 6; then
    base="$(home 5)/inhere"; mkdir -p "$base"
    for j in $(seq 0 19); do
        sub=$(printf '%s/maybehere%02d' "$base" "$j"); mkdir -p "$sub"
        for k in $(seq 1 3); do
            df=$(printf '%s/-file%d' "$sub" "$k")
            head -c $(( (RANDOM % 4000) + 50 )) /dev/urandom > "$df"
            chmod 644 "$df"
        done
        xf="$sub/spaces file"
        head -c 1033 /dev/urandom > "$xf"; chmod 755 "$xf"
    done
    target=$(printf '%s/maybehere07/.file2' "$base")
    { pw 6; yes '' | head -c $(( 1033 - 33 )); } > "$target"
    truncate -s 1033 "$target"
    chmod 644 "$target"
    chown -R "$(u 5):$(u 5)" "$base"
    find "$base" -type d -exec chmod 755 {} +
fi

# ---- L6 -> 7 : owned by <u7>:<u6>, 33 bytes, somewhere on disk ------------
if have 7; then
    d=/var/lib/dpkg/info; mkdir -p "$d"
    f="$d/$(u 7).password"
    pw 7 > "$f"                 # 32 chars + newline = 33 bytes
    chown "$(u 7):$(u 6)" "$f"; chmod 640 "$f"
fi

# ---- L7 -> 8 : grep for the word "millionth" -----------------------------
if have 8; then
    f="$(home 7)/data.txt"
    {
        for i in $(seq 1 4000); do printf '%s\t%s\n' "$(noise 8)" "$(noise 12)"; done
        printf 'millionth\t%s\n' "$(pw 8)"
        for i in $(seq 1 4000); do printf '%s\t%s\n' "$(noise 8)" "$(noise 12)"; done
    } | shuf > "$f"
    chown "$(u 7):$(u 7)" "$f"; chmod 644 "$f"
fi

# ---- L8 -> 9 : the only line that occurs exactly once ---------------------
if have 9; then
    f="$(home 8)/data.txt"
    {
        for i in $(seq 1 500); do t="$(noise 32)"; printf '%s\n%s\n' "$t" "$t"; done
        pw 9
    } | shuf > "$f"
    chown "$(u 8):$(u 8)" "$f"; chmod 644 "$f"
fi

# ---- L9 -> 10 : strings, preceded by several '=' -------------------------
if have 10; then
    f="$(home 9)/data.txt"
    {
        head -c 300 /dev/urandom
        printf '========== %s' "$(pw 10)"
        head -c 300 /dev/urandom
    } > "$f"
    chown "$(u 9):$(u 9)" "$f"; chmod 644 "$f"
fi

# ---- L10 -> 11 : base64 --------------------------------------------------
if have 11; then
    f="$(home 10)/data.txt"
    pw 11 | base64 > "$f"
    chown "$(u 10):$(u 10)" "$f"; chmod 644 "$f"
fi

echo "Planted levels 0..$(( MAX_LEVEL < 11 ? MAX_LEVEL : 11 )) for prefix '$PREFIX'"
