# The game "world": a real Debian box with real per-level users. Level
# progression is enforced by genuine Unix permissions, not by the TUI.
#
# Parameterized by USER_PREFIX / PASS_DIR so the same mechanics can be
# re-skinned per campaign (e.g. bandit0.. for the plain skin, node0.. for
# the fsociety skin). The techniques are identical; only the names change.
FROM debian:bookworm-slim

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        openssh-server \
        coreutils bsdmainutils binutils xxd file findutils grep gzip bzip2 tar \
        openssl netcat-openbsd nmap git cron vim less \
        ca-certificates curl unzip \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /run/sshd
RUN sed -i 's/^#\?Port .*/Port 2220/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?UsePAM.*/UsePAM yes/' /etc/ssh/sshd_config

# Campaign identity. Override at build time per skin.
ARG USER_PREFIX=bandit
ARG PASS_DIR=/etc/bandit_pass
ARG MAX_LEVEL=11
ENV USER_PREFIX=${USER_PREFIX} PASS_DIR=${PASS_DIR} MAX_LEVEL=${MAX_LEVEL}

# fsoc toolkit: fetch modern CLI tools off-PATH into /opt/fsoc/bin and install
# the `fsoc` package manager. Nothing is on PATH until a player runs `fsoc
# install <tier>`, so the box stays bare (and the wargame honest) by default.
COPY fsoc/ /opt/fsoc/
RUN chmod +x /opt/fsoc/fetch-tools.sh /opt/fsoc/fsoc \
    && /opt/fsoc/fetch-tools.sh \
    && ln -sf /opt/fsoc/fsoc /usr/local/bin/fsoc

COPY setup/ /opt/wargame/setup/
RUN chmod +x /opt/wargame/setup/*.sh && /opt/wargame/setup/provision.sh

EXPOSE 2220
CMD ["/opt/wargame/setup/entrypoint.sh"]
