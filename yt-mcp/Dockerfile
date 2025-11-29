FROM python:3.12-slim

# Install system dependencies for yt-dlp and ffmpeg
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Install uv
RUN pip install uv

WORKDIR /app

# Copy project files
COPY pyproject.toml .
COPY src/ src/

# Install dependencies
RUN uv sync

# Create directories
RUN mkdir -p transcripts .downloads

# Firefox cookies will be mounted at runtime
ENV FIREFOX_COOKIES_PATH=/firefox-cookies

# Default command
CMD ["uv", "run", "fastmcp", "run", "src/yt_transcribe/server.py"]
