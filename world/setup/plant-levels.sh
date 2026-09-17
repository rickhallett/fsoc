#!/bin/bash
# Plant each level's puzzle files. Runs as root at build time.
# banditN's home holds the password for bandit(N+1), extractable only
# via that level's intended technique. Guarded by MAX_LEVEL so the
# vertical-slice image (0..11) builds cleanly.
set -eu

MAX_LEVEL="${MAX_LEVEL:-11}"
PASSDIR=/etc/bandit_pass
pw() { cat "$PASSDIR/bandit$1"; }
have() { [ "$1" -le "$MAX_LEVEL" ]; }

# random printable filler that is NOT the password (for padding/decoys)
noise() { tr -dc 'A-Za-z0-9' </dev/urandom | head -c "$1"; }

# ---- L0 -> 1 : readme ------------------------------------------------------
if have 1; then
    f=/home/bandit0/readme
    pw 1 > "$f"; chown bandit0:bandit0 "$f"; chmod 644 "$f"
fi

# ---- L1 -> 2 : a file literally named "-" ---------------------------------
if have 2; then
    f='/home/bandit1/-'
    pw 2 > "$f"; chown bandit1:bandit1 "$f"; chmod 644 "$f"
fi

# ---- L2 -> 3 : spaces in the filename -------------------------------------
if have 3; then
    f='/home/bandit2/--spaces in this filename--'
    pw 3 > "$f"; chown bandit2:bandit2 "$f"; chmod 644 "$f"
fi

# ---- L3 -> 4 : hidden file in inhere/ -------------------------------------
if have 4; then
    d=/home/bandit3/inhere; mkdir -p "$d"
    f="$d/...Hiding-From-You"
    pw 4 > "$f"
    chown -R bandit3:bandit3 "$d"; chmod 755 "$d"; chmod 644 "$f"
fi

# ---- L4 -> 5 : only human-readable file in inhere/ ------------------------
if have 5; then
    d=/home/bandit4/inhere; mkdir -p "$d"
    real=$(( RANDOM % 10 ))
    for i in $(seq 0 9); do
        f=$(printf '%s/-file%02d' "$d" "$i")
        if [ "$i" -eq "$real" ]; then
            pw 5 > "$f"                       # ASCII text
        else
            head -c 200 /dev/urandom > "$f"   # binary noise
        fi
        chmod 644 "$f"
    done
    chown -R bandit4:bandit4 "$d"; chmod 755 "$d"
fi

# ---- L5 -> 6 : human-readable, 1033 bytes, not executable -----------------
if have 6; then
    base=/home/bandit5/inhere; mkdir -p "$base"
    # decoy tree
    for j in $(seq 0 19); do
        sub=$(printf '%s/maybehere%02d' "$base" "$j"); mkdir -p "$sub"
        for k in $(seq 1 3); do
            df=$(printf '%s/-file%d' "$sub" "$k")
            head -c $(( (RANDOM % 4000) + 50 )) /dev/urandom > "$df"
            chmod 644 "$df"
        done
        # an executable decoy so ! -executable actually matters
        xf="$sub/spaces file"
        head -c 1033 /dev/urandom > "$xf"; chmod 755 "$xf"
    done
    # the real one: exactly 1033 bytes, ASCII, mode 644. Password on its
    # own first line, padded to 1033 bytes with blank lines so `cat` reads
    # cleanly (matches the OverTheWire original).
    target=$(printf '%s/maybehere07/.file2' "$base")
    { pw 6; yes '' | head -c $(( 1033 - 33 )); } > "$target"
    truncate -s 1033 "$target"
    chmod 644 "$target"
    chown -R bandit5:bandit5 "$base"
    find "$base" -type d -exec chmod 755 {} +
fi

# ---- L6 -> 7 : owned by bandit7:bandit6, 33 bytes, somewhere on disk ------
if have 7; then
    d=/var/lib/dpkg/info; mkdir -p "$d"
    f="$d/bandit7.password"
    pw 7 > "$f"                 # 32 chars + newline = 33 bytes
    chown bandit7:bandit6 "$f"; chmod 640 "$f"
fi

# ---- L7 -> 8 : grep for the word "millionth" -----------------------------
if have 8; then
    f=/home/bandit7/data.txt
    {
        for i in $(seq 1 4000); do printf '%s\t%s\n' "$(noise 8)" "$(noise 12)"; done
        printf 'millionth\t%s\n' "$(pw 8)"
        for i in $(seq 1 4000); do printf '%s\t%s\n' "$(noise 8)" "$(noise 12)"; done
    } | shuf > "$f"
    chown bandit7:bandit7 "$f"; chmod 644 "$f"
fi

# ---- L8 -> 9 : the only line that occurs exactly once ---------------------
if have 9; then
    f=/home/bandit8/data.txt
    {
        for i in $(seq 1 500); do t="$(noise 32)"; printf '%s\n%s\n' "$t" "$t"; done
        pw 9                      # appears exactly once
    } | shuf > "$f"
    chown bandit8:bandit8 "$f"; chmod 644 "$f"
fi

# ---- L9 -> 10 : strings, preceded by several '=' -------------------------
if have 10; then
    f=/home/bandit9/data.txt
    {
        head -c 300 /dev/urandom
        printf '========== %s' "$(pw 10)"
        head -c 300 /dev/urandom
    } > "$f"
    chown bandit9:bandit9 "$f"; chmod 644 "$f"
fi

# ---- L10 -> 11 : base64 --------------------------------------------------
if have 11; then
    f=/home/bandit10/data.txt
    pw 11 | base64 > "$f"
    chown bandit10:bandit10 "$f"; chmod 644 "$f"
fi

echo "Planted levels 0..$(( MAX_LEVEL < 11 ? MAX_LEVEL : 11 ))"
