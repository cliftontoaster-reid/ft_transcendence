# Dockerfile - dev container
# Copyright (C) 2026 Clifton Toaster Reid
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

# Development image for the Ft_transcendence monorepo:
#   - Bun (frontend package manager / runner, matches CI `bun-version: latest`)
#   - Rust stable toolchain (matches CI `dtolnay/rust-toolchain@stable`)
#   - Buf CLI (proto generation / linting)
#   - PostgreSQL client (psql) for the compose `postgres` service

FROM mcr.microsoft.com/devcontainers/base:debian-12

ARG BUN_VERSION=latest
ARG RUSTUP_TOOLCHAIN=nightly

ENV DEBIAN_FRONTEND=noninteractive \
	BUN_INSTALL=/usr/local \
	RUSTUP_HOME=/usr/local/rustup \
	CARGO_HOME=/usr/local/cargo \
	PATH="/usr/local/cargo/bin:/usr/local/bin:${PATH}"

# System packages: native build deps for Rust crates, protoc (proto codegen), psql
RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
	build-essential \
	clang \
	lld \
	pkg-config \
	libssl-dev \
	protobuf-compiler \
	postgresql-client \
	&& apt-get clean \
	&& rm -rf /var/lib/apt/lists/*

# Bun (latest by default, matching CI; override with --build-arg BUN_VERSION)
RUN curl -fsSL https://bun.sh/install | bash \
	&& bun --version

# Rust toolchain (stable by default, matching CI; override with --build-arg RUSTUP_TOOLCHAIN)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
	| sh -s -- -y --profile default --no-modify-path --default-toolchain "${RUSTUP_TOOLCHAIN}" \
	&& rustup component add rustfmt clippy rust-analyzer \
	&& rustc --version && cargo --version

# Buf CLI (latest release)
RUN curl -fsSL \
	"https://github.com/bufbuild/buf/releases/latest/download/buf-Linux-$(uname -m)" \
	-o /usr/local/bin/buf \
	&& chmod +x /usr/local/bin/buf \
	&& buf --version

# Let the non-root `vscode` user manage the toolchain and update components
RUN chown -R vscode:vscode "${CARGO_HOME}" "${RUSTUP_HOME}"