# fsoc: comfort
if command -v zoxide >/dev/null 2>&1; then
  eval "$(zoxide init bash)"
  zd() { if (($#==0)); then builtin cd ~; elif [[ -d $1 ]]; then builtin cd "$@"; else z "$@" || return; pwd; fi; }
  alias cd=zd
fi
if command -v fzf >/dev/null 2>&1; then
  source <(fzf --bash) 2>/dev/null || true
  ff() { fzf --preview 'bat --color=always -- {} 2>/dev/null || cat -- {}' "$@"; }
fi
alias ..='cd ..'
alias ...='cd ../..'
