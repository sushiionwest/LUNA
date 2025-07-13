# Luna Agent - Real VM Implementation
# Production-ready container with full automation stack

FROM ubuntu:22.04

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1
ENV DISPLAY=:99
ENV LUNA_HOME=/opt/luna-agent

# Install system dependencies
RUN apt-get update && apt-get install -y \
    # Python and development tools
    python3 \
    python3-pip \
    python3-venv \
    python3-dev \
    build-essential \
    # Web browsers and automation
    chromium-browser \
    firefox \
    wget \
    curl \
    # Display and graphics
    xvfb \
    x11vnc \
    fluxbox \
    # Image processing
    libopencv-dev \
    python3-opencv \
    tesseract-ocr \
    tesseract-ocr-eng \
    # System tools
    htop \
    vim \
    git \
    unzip \
    # Audio/Video processing
    ffmpeg \
    # Network tools
    net-tools \
    openssh-server \
    # Clean up
    && rm -rf /var/lib/apt/lists/*

# Install Chrome for Selenium
RUN wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | apt-key add - \
    && echo "deb [arch=amd64] https://dl.google.com/linux/chrome/deb/ stable main" > /etc/apt/sources.list.d/google-chrome.list \
    && apt-get update \
    && apt-get install -y google-chrome-stable \
    && rm -rf /var/lib/apt/lists/*

# Install ChromeDriver
RUN CHROMEDRIVER_VERSION=$(curl -sS chromedriver.storage.googleapis.com/LATEST_RELEASE) \
    && wget -O /tmp/chromedriver.zip "https://chromedriver.storage.googleapis.com/${CHROMEDRIVER_VERSION}/chromedriver_linux64.zip" \
    && unzip /tmp/chromedriver.zip -d /tmp/ \
    && mv /tmp/chromedriver /usr/local/bin/chromedriver \
    && chmod +x /usr/local/bin/chromedriver \
    && rm /tmp/chromedriver.zip

# Create Luna user and directories
RUN useradd -m -s /bin/bash luna \
    && mkdir -p $LUNA_HOME/{screenshots,downloads,logs,temp,ui} \
    && chown -R luna:luna $LUNA_HOME

# Install Python dependencies
COPY luna-agent/requirements.txt /tmp/requirements.txt
RUN pip3 install --no-cache-dir -r /tmp/requirements.txt

# Install Playwright browsers
RUN pip3 install playwright \
    && playwright install chromium firefox webkit \
    && playwright install-deps

# Copy Luna Agent application
COPY luna-agent/ $LUNA_HOME/
RUN chown -R luna:luna $LUNA_HOME

# Create startup scripts
COPY scripts/ /opt/scripts/
RUN chmod +x /opt/scripts/*.sh

# Configure SSH (for remote access)
RUN echo 'luna:luna123' | chpasswd \
    && usermod -aG sudo luna \
    && sed -i 's/#PasswordAuthentication yes/PasswordAuthentication yes/' /etc/ssh/sshd_config \
    && sed -i 's/#Port 22/Port 22222/' /etc/ssh/sshd_config

# Configure VNC server
RUN mkdir -p /home/luna/.vnc \
    && echo 'luna123' | vncpasswd -f > /home/luna/.vnc/passwd \
    && chmod 600 /home/luna/.vnc/passwd \
    && chown -R luna:luna /home/luna/.vnc

# Create Luna service configuration
COPY configs/luna-agent.service /etc/systemd/system/
RUN systemctl enable luna-agent

# Expose ports
EXPOSE 8080    # Luna API
EXPOSE 3000    # Luna UI
EXPOSE 22222   # SSH
EXPOSE 5900    # VNC
EXPOSE 6080    # noVNC (web VNC)

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/status || exit 1

# Switch to Luna user
USER luna
WORKDIR $LUNA_HOME

# Create startup script for Luna user
RUN echo '#!/bin/bash' > /home/luna/start-luna.sh \
    && echo 'export DISPLAY=:99' >> /home/luna/start-luna.sh \
    && echo 'cd $LUNA_HOME' >> /home/luna/start-luna.sh \
    && echo 'python3 main.py' >> /home/luna/start-luna.sh \
    && chmod +x /home/luna/start-luna.sh

# Default command
CMD ["/opt/scripts/start-luna-vm.sh"]