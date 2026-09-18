
# --- wargamez terminal theme (someone tricked this box out) ---
export TERM=xterm-256color
force_color_prompt=yes
PS1='\[\e[38;5;179m\]\u\[\e[38;5;242m\]@\[\e[38;5;173m\]\h \[\e[38;5;245m\]\w \[\e[38;5;166m\]\$\[\e[0m\] '
alias ls='ls --color=auto'
alias ll='ls -la --color=auto'
alias grep='grep --color=auto'
export LS_COLORS='di=1;38;5;179:ln=38;5;109:ex=1;38;5;71:*.gz=38;5;96:*.bz2=38;5;96:*.tar=38;5;96:*.txt=38;5;250'

# fsoc: expose per-node installed tools + their integrations (bare until installed)
export PATH="$HOME/.local/bin:$PATH"
[ -r "$HOME/.fsoc/init.bash" ] && source "$HOME/.fsoc/init.bash"
