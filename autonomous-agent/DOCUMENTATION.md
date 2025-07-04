# Autonomous Agent Documentation

## Overview

The Autonomous Agent is a comprehensive computer use automation system that provides advanced capabilities for screen capture, window management, social media automation, and task execution. Built with a modern TypeScript/React stack, it features a real-time dashboard for monitoring and controlling all agent activities.

## 🚀 Features

### Core Agent Capabilities
- **Autonomous Task Execution**: Intelligent task planning and execution with queuing system
- **Screen Capture & Control**: Advanced screenshot functionality with image comparison
- **Window Management**: Install, manage, and control applications and windows
- **Social Media Automation**: AI-powered content generation and posting with Microsoft Vision integration
- **Real-time Monitoring**: Live dashboard with WebSocket communication
- **Configuration Management**: Centralized settings for all services and APIs

### Dashboard Features
- **Agent Control**: Start/stop agent, monitor status and performance
- **Task Manager**: Create, monitor, and manage automated tasks
- **Screen Capture**: Live screen capture with gallery and comparison tools
- **Social Media**: Content composition, AI generation, and multi-platform posting
- **Window Installer**: Browse packages, install applications, manage windows
- **System Monitor**: Resource usage, service status, and health monitoring
- **Configuration Panel**: API keys, agent behavior, and system settings

## 🏗️ Architecture

### Backend Services
- **AgentService**: Core automation logic and task execution
- **DatabaseService**: SQLite database for persistent storage
- **ScreenCaptureService**: Screenshot functionality and image processing
- **SocialMediaService**: Social media integration and content generation
- **WindowInstallerService**: Application installation and window management
- **ConfigService**: Configuration management and validation
- **SocketHandler**: Real-time WebSocket communication

### Frontend Components
- **Dashboard Layout**: Main application with tabbed interface
- **Agent Control**: Status monitoring and control panel
- **Task Manager**: Task creation and monitoring interface
- **Screen Capture**: Live capture and image gallery
- **Social Media**: Content creation and posting interface
- **Window Installer**: Package browser and installation manager
- **System Monitor**: Health and performance dashboard
- **Configuration Panel**: Settings and API configuration

### API Routes
- `/api/agent/*` - Agent control and task management
- `/api/system/*` - System status and monitoring
- `/api/social/*` - Social media operations
- `/api/installer/*` - Application installation and window management

## 📦 Technology Stack

### Backend
- **Node.js** with **Express.js** - Web server framework
- **Socket.io** - Real-time WebSocket communication
- **SQLite3** - Database for persistent storage
- **TypeScript** - Type-safe JavaScript development
- **Puppeteer** - Browser automation (future integration)
- **Sharp** - Image processing and manipulation
- **Microsoft Vision API** - AI-powered image analysis

### Frontend
- **React 19** - Modern UI framework
- **TypeScript** - Type-safe development
- **Vite** - Fast build tool and dev server
- **Tailwind CSS V4** - Utility-first styling
- **ShadCN UI** - High-quality component library
- **Recharts** - Data visualization and metrics
- **Lucide React** - Beautiful icon library

### Package Management
- **Bun** - Fast JavaScript package manager and runtime

## 🛠️ Installation & Setup

### Prerequisites
- Node.js 18+ or Bun 1.0+
- Linux-based operating system (for window management)
- Git for version control

### Installation Steps

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd autonomous-agent
   ```

2. **Install dependencies**
   ```bash
   bun install
   ```

3. **Environment setup**
   ```bash
   cp .env.example .env
   # Edit .env with your API keys and configuration
   ```

4. **Required API Keys**
   - Microsoft Vision API key
   - Twitter API credentials
   - Instagram API credentials (optional)
   - LinkedIn API credentials (optional)

### Environment Variables

```env
# Server Configuration
PORT=3001
NODE_ENV=development
DATABASE_PATH=./data/agent.db

# Microsoft Vision API
MICROSOFT_VISION_API_KEY=your_vision_api_key
MICROSOFT_VISION_ENDPOINT=your_vision_endpoint

# Social Media APIs
TWITTER_API_KEY=your_twitter_api_key
TWITTER_API_SECRET=your_twitter_api_secret
TWITTER_ACCESS_TOKEN=your_twitter_access_token
TWITTER_ACCESS_TOKEN_SECRET=your_twitter_access_token_secret

# Agent Configuration
AGENT_MAX_CONCURRENT_TASKS=5
AGENT_TASK_TIMEOUT=300000
AGENT_RETRY_ATTEMPTS=3

# Security
JWT_SECRET=your_jwt_secret
BCRYPT_ROUNDS=12

# Frontend
FRONTEND_URL=http://localhost:5173
```

## 🚀 Running the Application

### Development Mode

1. **Start the backend server**
   ```bash
   bun run server
   ```

2. **Start the frontend development server**
   ```bash
   bun run dev
   ```

3. **Access the dashboard**
   - Frontend: http://localhost:5173
   - Backend API: http://localhost:3001
   - Health Check: http://localhost:3001/health

### Production Mode

1. **Build the application**
   ```bash
   bun run build
   bun run build:server
   ```

2. **Start production server**
   ```bash
   bun run start
   ```

## 📖 Usage Guide

### Agent Control
1. Navigate to the **Overview** tab
2. Use the **Start/Stop** buttons to control the agent
3. Monitor real-time status and performance metrics
4. View active tasks and system health

### Task Management
1. Go to the **Tasks** tab
2. Click **Create Task** to add new automation
3. Select task type (screen capture, social media, automation, etc.)
4. Configure task parameters and schedule
5. Monitor task progress and results

### Screen Capture
1. Access the **Screen** tab
2. Configure capture options (format, quality, region)
3. Take manual screenshots or enable live capture
4. View captured images in the gallery
5. Compare screenshots for change detection

### Social Media Automation
1. Visit the **Social** tab
2. Configure platform connections (Twitter, Instagram, LinkedIn)
3. Compose content or use AI generation
4. Upload images for analysis and enhancement
5. Schedule or post immediately to selected platforms

### Window Installer
1. Open the **Installer** tab
2. Browse available packages by category
3. Search for specific applications
4. Install packages with custom options
5. Manage installed applications and windows
6. Control window states (minimize, maximize, close)

### System Monitoring
1. Check the **System** tab for health overview
2. Monitor CPU, memory, and storage usage
3. View service status and logs
4. Track performance metrics over time

### Configuration
1. Access the **Config** tab
2. Update API keys and credentials
3. Modify agent behavior settings
4. Configure automation rules
5. Test service connections

## 🔧 API Reference

### Agent Endpoints

```typescript
// Start the agent
POST /api/agent/start

// Stop the agent
POST /api/agent/stop

// Get agent status
GET /api/agent/status

// Add a new task
POST /api/agent/tasks
{
  "type": "screen_capture" | "social_media" | "automation" | "analysis" | "installation" | "custom",
  "priority": "low" | "medium" | "high",
  "params": { ... }
}

// Get all tasks
GET /api/agent/tasks

// Cancel a task
DELETE /api/agent/tasks/:taskId

// Add automation rule
POST /api/agent/automation/rules
{
  "name": "string",
  "trigger": { ... },
  "actions": [...],
  "enabled": boolean
}
```

### System Endpoints

```typescript
// Get system status
GET /api/system/status

// Get activity logs
GET /api/system/logs

// Get system metrics
GET /api/system/metrics

// Capture screenshot
POST /api/system/screenshot
{
  "filename": "string",
  "format": "png" | "jpg",
  "quality": 1-100
}

// Compare screenshots
POST /api/system/screenshot/compare
{
  "image1": "string",
  "image2": "string"
}
```

### Social Media Endpoints

```typescript
// Analyze image
POST /api/social/analyze
// FormData with image file

// Generate content
POST /api/social/generate
{
  "type": "caption" | "hashtags" | "description",
  "context": "string",
  "platform": "twitter" | "instagram" | "linkedin"
}

// Post to platform
POST /api/social/post
{
  "platform": "twitter" | "instagram" | "linkedin",
  "content": "string",
  "images": ["string"],
  "scheduledTime": "ISO string" // optional
}
```

### Installer Endpoints

```typescript
// Get available packages
GET /api/installer/packages?category=string&platform=string

// Search packages
GET /api/installer/packages/search?query=string

// Install package
POST /api/installer/install
{
  "packageId": "string",
  "options": {
    "autoStart": boolean,
    "createDesktopShortcut": boolean,
    "customInstallPath": "string"
  }
}

// Get installation status
GET /api/installer/install/:taskId

// Get installed applications
GET /api/installer/installed

// Get windows
GET /api/installer/windows

// Manage window
POST /api/installer/windows/:windowId/:action
// action: minimize, maximize, close, focus
```

## 🔌 WebSocket Events

### Client → Server
- `agent:start` - Start the agent
- `agent:stop` - Stop the agent
- `status:get` - Request current status
- `task:add` - Add new task
- `screenshot:take` - Take screenshot

### Server → Client
- `agent:status` - Agent status update
- `status:data` - System status update
- `task:created` - New task created
- `task:updated` - Task status changed
- `task:completed` - Task finished
- `screenshot:captured` - Screenshot available
- `error` - Error notification

## 🛡️ Security Considerations

### API Security
- JWT token authentication for sensitive operations
- API rate limiting to prevent abuse
- Input validation and sanitization
- Secure credential storage

### System Security
- Sandboxed task execution
- Limited file system access
- Secure screenshot storage
- Encrypted configuration files

### Network Security
- CORS configuration for cross-origin requests
- HTTPS enforcement in production
- Secure WebSocket connections
- API endpoint protection

## 🔍 Troubleshooting

### Common Issues

1. **Agent won't start**
   - Check database connection
   - Verify required dependencies
   - Review system logs for errors

2. **Screenshots failing**
   - Ensure X11 is available (Linux)
   - Check screen capture permissions
   - Verify screenshot directory exists

3. **Social media posting errors**
   - Validate API credentials
   - Check rate limits
   - Verify content format compliance

4. **Package installation fails**
   - Check system package managers
   - Verify installation permissions
   - Review package compatibility

### Debug Mode
Enable verbose logging by setting:
```env
NODE_ENV=development
LOG_LEVEL=debug
```

### Health Checks
Monitor system health at:
- http://localhost:3001/health
- Dashboard System tab
- Real-time WebSocket status

## 📈 Performance Optimization

### Database Optimization
- Regular cleanup of old logs and captures
- Index optimization for frequent queries
- Connection pooling for concurrent access

### Memory Management
- Automatic cleanup of processed images
- Task queue size limits
- Regular garbage collection

### Network Optimization
- Image compression for transfers
- WebSocket message batching
- API response caching

## 🤝 Contributing

### Development Setup
1. Fork the repository
2. Create a feature branch
3. Follow TypeScript best practices
4. Add tests for new functionality
5. Update documentation
6. Submit a pull request

### Code Style
- Use TypeScript strict mode
- Follow ESLint configuration
- Implement proper error handling
- Add JSDoc comments for functions
- Use meaningful variable names

### Testing
- Unit tests for services
- Integration tests for API endpoints
- E2E tests for critical workflows
- Performance tests for heavy operations

## 📄 License

This project is licensed under the MIT License. See LICENSE file for details.

## 📞 Support

For issues, questions, or contributions:
- Create an issue on GitHub
- Check the troubleshooting guide
- Review the API documentation
- Contact the development team

## 🔄 Changelog

### v1.0.0 - Initial Release
- Core agent functionality
- Comprehensive dashboard
- Screen capture and control
- Social media automation
- Window installer
- Real-time monitoring
- Configuration management
- Documentation and guides

---

**Built with ❤️ using modern web technologies**