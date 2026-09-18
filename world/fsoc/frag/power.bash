# fsoc: power (assisted mode) - eza shadows ls; rg/fd/sd/lazygit by name.
# grep/find/cat are left untouched so scripts and the core lessons still work.
if command -v eza >/dev/null 2>&1; then
  alias ls='eza -lh --group-directories-first --icons=never'
  alias lsa='ls -a'
  alias lt='eza --tree --level=2 --long'
  alias lta='lt -a'
fi
