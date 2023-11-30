FROM rust:latest
WORKDIR /usr/src/tiktokx-telegram

# Install yt-dlp
RUN apt-get update && apt-get install -y \
    curl \
 && rm -rf /var/lib/apt/lists/*

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

ENV YT_DLP=/usr/local/bin/yt-dlp
ENV TELEGRAM_TOKEN=YOUR_TELEGRAM_TOKEN

COPY Cargo.toml Cargo.toml
COPY src src
RUN cargo build --release
CMD ["./target/release/tiktokx-telegram"]
