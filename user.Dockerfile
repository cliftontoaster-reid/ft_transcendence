# Dockerfile - user
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

FROM rust:1-bookworm AS builder

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
	build-essential \
	pkg-config \
	libpq-dev \
	libssl-dev \
	protobuf-compiler \
	libprotobuf-dev \
	&& apt-get clean \
	&& rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY ./server/Cargo.toml ./server/Cargo.lock ./server/diesel.toml ./server/
COPY ./server/crates ./server/crates
COPY ./server/plugins ./server/plugins
COPY ./server/migrations ./server/migrations
COPY ./proto ./proto
COPY ./third_party/googleapis ./third_party/googleapis

RUN mkdir -p /workspace/third_party/googleapis/google/protobuf \
	&& cp /usr/include/google/protobuf/*.proto /workspace/third_party/googleapis/google/protobuf/

WORKDIR /workspace/server
RUN cargo build --release --locked -p dionysus_user

FROM debian:bookworm-slim AS production

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
	ca-certificates \
	curl \
	libpq5 \
	&& apt-get clean \
	&& rm -rf /var/lib/apt/lists/*

WORKDIR /app

ENV USER_LISTEN_ADDR=0.0.0.0:50052

COPY --from=builder /workspace/server/target/release/dionysus_user /usr/local/bin/dionysus_user

EXPOSE 50052

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD bash -c 'echo > /dev/tcp/127.0.0.1/50052'

CMD ["dionysus_user"]
