# The Bandit "game world": a real Debian box with real bandit users.
# Level progression is enforced by genuine Unix permissions, not by the
# TUI. banditN can only reach banditN+1 via the intended technique.
FROM debian:bookworm-slim

# Toolchain the levels need. Kept lean; extend as later levels are added.
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        openssh-server \
        coreutils bsdmainutils binutils xxd file findutils grep gzip bzip2 tar \
        openssl netcat-openbsd nmap git cron vim less \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# sshd needs this dir; the game listens on 2220 like the real thing.
RUN mkdir -p /run/sshd
RUN sed -i 's/^#\?Port .*/Port 2220/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?UsePAM.*/UsePAM yes/' /etc/ssh/sshd_config \
    && echo "AllowUsers bandit0 bandit1 bandit2 bandit3 bandit4 bandit5 bandit6 bandit7 bandit8 bandit9 bandit10 bandit11 bandit12 bandit13 bandit14" >> /etc/ssh/sshd_config

# How many bandit levels this image provisions (0..N). The vertical slice
# builds through 11; bump this as later levels' setup is filled in.
ARG MAX_LEVEL=11
ENV MAX_LEVEL=${MAX_LEVEL}

COPY setup/ /opt/wargame/setup/
RUN chmod +x /opt/wargame/setup/*.sh && /opt/wargame/setup/provision.sh

EXPOSE 2220
CMD ["/opt/wargame/setup/entrypoint.sh"]
