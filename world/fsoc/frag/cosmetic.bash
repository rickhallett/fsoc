# fsoc: cosmetic
if command -v starship >/dev/null 2>&1; then
  export STARSHIP_CONFIG=/opt/fsoc/starship.toml
  eval "$(starship init bash)"
fi
if command -v bat >/dev/null 2>&1; then
  export BAT_THEME="${BAT_THEME:-ansi}"
  export MANPAGER="sh -c 'col -bx | bat -l man -p'"
fi
