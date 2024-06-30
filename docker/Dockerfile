FROM node:20 AS vue-build
WORKDIR /app
COPY onceuponai-ui/package*.json ./
RUN npm install
COPY onceuponai-ui/ .
RUN npm run build

FROM nvidia/cuda:12.3.1-devel-ubuntu22.04 AS rust-build

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && curl https://sh.rustup.rs -sSf | sh -s -- -y

ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /app
COPY onceuponai-backend/ .
WORKDIR /app/onceuponai
RUN cargo build --release

#FROM debian:buster-slim
#
#RUN apt-get update && apt-get install -y \
#    libssl1.1 \
#    ca-certificates \
#    && rm -rf /var/lib/apt/lists/*
#
#WORKDIR /app
#COPY --from=vue-build /app/dist ./frontend
#COPY --from=rust-build /app/target/release/onceuponai .
#
#EXPOSE 8080
#
#CMD ["./onceuponai"]
#