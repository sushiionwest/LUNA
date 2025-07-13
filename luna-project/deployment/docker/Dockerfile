# Luna Agent Development Container
# Multi-stage build for efficient development and testing

# Stage 1: Base development environment
FROM ubuntu:22.04 as base

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive
ENV NODE_VERSION=18
ENV PYTHON_VERSION=3.10

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    python3 \
    python3-pip \
    python3-venv \
    virtualbox \
    xvfb \
    chromium-browser \
    firefox \
    openssh-client \
    jq \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash - \
    && apt-get install -y nodejs

# Install Bun (for faster package management)
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

# Stage 2: Luna development environment
FROM base as luna-dev

WORKDIR /luna-project

# Copy package files
COPY package*.json ./
COPY testing/package*.json ./testing/
COPY vm-assets/ ./vm-assets/
COPY installer/ ./installer/

# Install dependencies
RUN npm install && \
    cd testing && npm install

# Install Python dependencies
RUN pip3 install \
    selenium \
    playwright \
    pytest \
    fastapi \
    uvicorn

# Install Playwright browsers
RUN npx playwright install chromium firefox webkit \
    && npx playwright install-deps

# Create Luna directories
RUN mkdir -p /opt/luna-agent/{screenshots,downloads,logs,temp}

# Copy source code
COPY . .

# Set up permissions
RUN chmod +x vm-assets/scripts/*.sh

# Expose ports
EXPOSE 8080 3000 22222 5900

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/status || exit 1

# Default command
CMD ["npm", "run", "dev"]

# Stage 3: Testing environment
FROM luna-dev as luna-test

# Copy test files
COPY testing/ ./testing/

# Run tests by default
CMD ["npm", "run", "test"]

# Stage 4: Production container (for Luna Agent server)
FROM base as luna-prod

WORKDIR /opt/luna-agent

# Install only production dependencies
RUN pip3 install \
    fastapi \
    uvicorn \
    selenium \
    playwright \
    opencv-python \
    pillow \
    requests \
    websockets \
    pyautogui \
    psutil \
    schedule \
    python-dotenv \
    aiofiles \
    httpx

# Install Playwright browsers
RUN npx playwright install chromium firefox webkit --with-deps

# Create Luna user
RUN useradd -m -s /bin/bash luna && \
    usermod -aG sudo luna && \
    chown -R luna:luna /opt/luna-agent

# Copy Luna Agent application
COPY vm-assets/software/luna-agent/ ./

# Switch to Luna user
USER luna

# Create Python virtual environment
RUN python3 -m venv luna-env

# Expose ports
EXPOSE 8080 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/status || exit 1

# Start Luna Agent
CMD ["./luna-env/bin/python", "main.py"]