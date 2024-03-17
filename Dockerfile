FROM rust:1.74
WORKDIR /usr/src/tiktokx-telegram

# Install yt-dlp
RUN apt-get update && apt-get install -y \
    curl \
 && rm -rf /var/lib/apt/lists/*

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

COPY Cargo.toml Cargo.toml
COPY src src
COPY cookies.txt cookies.txt
RUN cargo build --release
CMD ["./target/release/tiktokx-telegram"]
