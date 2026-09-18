#!/bin/bash
# Build-time: fetch modern CLI tools as aarch64 binaries into /opt/fsoc/bin
# (deliberately OFF the default PATH). fsoc exposes them per tier at runtime,
# so a "bare" box only has the wargame baseline. Best-effort per tool: a
# failed fetch just leaves that tool unresolved, and fsoc reports it.
set -u
DEST=/opt/fsoc/bin
mkdir -p "$DEST"

# name|repo|asset-name-regex|type(tar|zip|raw)  (binary basename == name)
TABLE='
starship|starship/starship|starship-aarch64-unknown-linux-musl\.tar\.gz|tar
eza|eza-community/eza|eza_aarch64-unknown-linux-gnu\.tar\.gz|tar
bat|sharkdp/bat|bat-v.*-aarch64-unknown-linux-gnu\.tar\.gz|tar
glow|charmbracelet/glow|glow_.*_Linux_arm64\.tar\.gz|tar
gum|charmbracelet/gum|gum_.*_Linux_arm64\.tar\.gz|tar
fastfetch|fastfetch-cli/fastfetch|fastfetch-linux-aarch64\.tar\.gz|tar
tldr|dbrgn/tealdeer|tealdeer-linux-aarch64-musl$|raw
fzf|junegunn/fzf|fzf-.*-linux_arm64\.tar\.gz|tar
zoxide|ajeetdsouza/zoxide|zoxide-.*-aarch64-unknown-linux-musl\.tar\.gz|tar
yazi|sxyazi/yazi|yazi-aarch64-unknown-linux-gnu\.zip|zip
atuin|atuinsh/atuin|atuin-.*-aarch64-unknown-linux-gnu\.tar\.gz|tar
dust|bootandy/dust|dust-.*-aarch64-unknown-linux-gnu\.tar\.gz|tar
rg|BurntSushi/ripgrep|ripgrep-.*-aarch64-unknown-linux-gnu\.tar\.gz|tar
fd|sharkdp/fd|fd-v.*-aarch64-unknown-linux-gnu\.tar\.gz|tar
sd|chmln/sd|sd-.*-aarch64-unknown-linux-gnu\.tar\.gz|tar
lazygit|jesseduffield/lazygit|lazygit_.*_Linux_arm64\.tar\.gz|tar
gh|cli/cli|gh_.*_linux_arm64\.tar\.gz|tar
'

while IFS='|' read -r name repo re type; do
    [ -z "$name" ] && continue
    case "$name" in \#*) continue;; esac
    url=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" \
        | grep -oE '"browser_download_url": *"[^"]+"' \
        | sed 's/.*": *"//; s/"$//' \
        | grep -E "/${re}" | head -1)
    if [ -z "$url" ]; then echo "unresolved: $name"; continue; fi
    tmp=$(mktemp -d); f="$tmp/asset"
    if ! curl -fsSL "$url" -o "$f"; then echo "dl-fail: $name"; rm -rf "$tmp"; continue; fi
    case "$type" in
        raw) install -m755 "$f" "$DEST/$name"; echo "ok: $name"; rm -rf "$tmp"; continue;;
        tar) tar xzf "$f" -C "$tmp" 2>/dev/null;;
        zip) (cd "$tmp" && unzip -qq asset) 2>/dev/null;;
    esac
    bin=$(find "$tmp" -type f -name "$name" 2>/dev/null | head -1)
    if [ -n "$bin" ]; then install -m755 "$bin" "$DEST/$name"; echo "ok: $name"
    else echo "nobin: $name"; fi
    rm -rf "$tmp"
done <<EOF
$TABLE
EOF

# mise via its own installer (single binary)
if curl -fsSL https://mise.run | MISE_INSTALL_PATH="$DEST/mise" sh >/dev/null 2>&1; then
    echo "ok: mise"
else
    echo "unresolved: mise"
fi

echo "--- /opt/fsoc/bin ---"
ls -1 "$DEST" 2>/dev/null
