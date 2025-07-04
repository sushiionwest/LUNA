# Contributing to Autonomous Agent

We love your input! We want to make contributing to the Autonomous Agent as easy and transparent as possible, whether it's:

- Reporting a bug
- Discussing the current state of the code
- Submitting a fix
- Proposing new features
- Becoming a maintainer

## 🚀 Development Process

We use GitHub to host code, track issues and feature requests, and accept pull requests.

### Pull Requests
Pull requests are the best way to propose changes to the codebase. We actively welcome your pull requests:

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. If you've changed APIs, update the documentation
4. Ensure the test suite passes
5. Make sure your code lints
6. Issue that pull request!

## 🐛 Report Bugs Using GitHub Issues

We use GitHub issues to track public bugs. Report a bug by [opening a new issue](../../issues/new).

**Great Bug Reports** tend to have:

- A quick summary and/or background
- Steps to reproduce
  - Be specific!
  - Give sample code if you can
- What you expected would happen
- What actually happens
- Notes (possibly including why you think this might be happening, or stuff you tried that didn't work)

## 💻 Development Setup

### Prerequisites
- Node.js 18+ or Bun 1.0+
- Git
- Linux-based OS (for full functionality)

### Setting Up Development Environment

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/your-username/autonomous-agent.git
   cd autonomous-agent
   ```

2. **Install dependencies**
   ```bash
   bun install
   ```

3. **Set up environment variables**
   ```bash
   cp .env.example .env
   # Edit .env with your development configuration
   ```

4. **Start development servers**
   ```bash
   # Terminal 1: Backend
   bun run server
   
   # Terminal 2: Frontend
   bun run dev
   ```

5. **Verify setup**
   ```bash
   # Test project structure
   node test-server.cjs
   
   # Check health endpoint
   curl http://localhost:3001/health
   ```

## 📝 Code Style Guidelines

### TypeScript/JavaScript
- Use TypeScript for all new code
- Follow the existing ESLint configuration
- Use meaningful variable and function names
- Add JSDoc comments for public APIs
- Prefer `const` over `let`, avoid `var`
- Use async/await over Promises where possible

### React Components
- Use functional components with hooks
- Implement proper error boundaries
- Add proper TypeScript interfaces for props
- Use semantic HTML elements
- Implement accessibility best practices

### Backend Services
- Follow the existing service pattern
- Implement proper error handling
- Add input validation for all endpoints
- Use structured logging
- Follow RESTful API conventions

### Example Code Style

```typescript
/**
 * Analyzes an image using Microsoft Vision API
 * @param imagePath - Path to the image file
 * @param options - Analysis options
 * @returns Promise with analysis results
 */
export async function analyzeImage(
  imagePath: string,
  options: AnalysisOptions = {}
): Promise<AnalysisResult> {
  try {
    // Implementation here
    const result = await visionClient.analyzeImage(imagePath, options);
    return {
      success: true,
      data: result,
      timestamp: new Date()
    };
  } catch (error) {
    logger.error('Image analysis failed:', error);
    throw new Error(`Analysis failed: ${error.message}`);
  }
}
```

## 🧪 Testing Guidelines

### Unit Tests
- Write tests for all new functions and methods
- Use descriptive test names
- Follow the AAA pattern (Arrange, Act, Assert)
- Mock external dependencies

### Integration Tests
- Test API endpoints end-to-end
- Verify database operations
- Test error scenarios

### Example Test

```typescript
describe('ScreenCaptureService', () => {
  let service: ScreenCaptureService;

  beforeEach(() => {
    service = new ScreenCaptureService();
  });

  it('should capture screenshot with default options', async () => {
    // Arrange
    const options = { format: 'png', quality: 90 };

    // Act
    const result = await service.captureScreen(options);

    // Assert
    expect(result.success).toBe(true);
    expect(result.filename).toMatch(/\.png$/);
    expect(fs.existsSync(result.path)).toBe(true);
  });
});
```

## 📚 Documentation Standards

### Code Documentation
- Add JSDoc comments for all public functions
- Include parameter types and return types
- Provide usage examples for complex functions
- Document any side effects or prerequisites

### API Documentation
- Update API documentation for new endpoints
- Include request/response examples
- Document error codes and messages
- Add authentication requirements

### README Updates
- Keep feature lists current
- Update installation instructions
- Add new configuration options
- Include troubleshooting for common issues

## 🏗️ Architecture Guidelines

### Adding New Services
1. Create service in `src/server/services/`
2. Implement proper error handling
3. Add database models if needed
4. Create API routes in `src/server/routes/`
5. Add frontend components in `src/components/`
6. Update main App.tsx and server index.ts

### Service Structure
```typescript
export class NewService {
  private config: ServiceConfig;
  private database: DatabaseService;

  constructor(config: ServiceConfig, database: DatabaseService) {
    this.config = config;
    this.database = database;
  }

  public async initialize(): Promise<void> {
    // Initialization logic
  }

  public async performAction(params: ActionParams): Promise<ActionResult> {
    // Service logic with proper error handling
  }
}
```

### Frontend Components
```typescript
interface ComponentProps {
  socket?: Socket;
  onUpdate?: (data: any) => void;
}

export const NewComponent: React.FC<ComponentProps> = ({ socket, onUpdate }) => {
  const [state, setState] = useState<ComponentState>({});

  useEffect(() => {
    // Setup socket listeners
    if (socket) {
      socket.on('event', handleEvent);
      return () => socket.off('event', handleEvent);
    }
  }, [socket]);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Component Title</CardTitle>
      </CardHeader>
      <CardContent>
        {/* Component content */}
      </CardContent>
    </Card>
  );
};
```

## 🔄 Git Workflow

### Branch Naming
- `feature/description` - New features
- `bugfix/description` - Bug fixes
- `hotfix/description` - Critical fixes
- `docs/description` - Documentation updates

### Commit Messages
Follow the [Conventional Commits](https://conventionalcommits.org/) specification:

```
type(scope): brief description

Detailed explanation if needed

Fixes #issue_number
```

Examples:
- `feat(agent): add task retry mechanism`
- `fix(ui): resolve dashboard layout issue`
- `docs(api): update installation instructions`
- `test(social): add unit tests for content generation`

### Pull Request Process
1. **Create descriptive PR title and description**
2. **Reference related issues**
3. **Add screenshots for UI changes**
4. **Ensure all checks pass**
5. **Request review from maintainers**

## 🏆 Recognition

### Contributors
All contributors will be recognized in:
- README.md contributors section
- Release notes for significant contributions
- GitHub contributor graphs

### Becoming a Maintainer
Regular contributors who demonstrate:
- Consistent quality contributions
- Good understanding of the codebase
- Helpful code reviews
- Community engagement

May be invited to become maintainers with additional responsibilities:
- Reviewing pull requests
- Triaging issues
- Release management
- Mentoring new contributors

## 📋 Issue Labels

We use these labels to organize issues:

- `bug` - Something isn't working
- `enhancement` - New feature or request
- `documentation` - Improvements or additions to docs
- `good first issue` - Good for newcomers
- `help wanted` - Extra attention is needed
- `question` - Further information is requested
- `priority:high` - High priority issue
- `priority:medium` - Medium priority issue
- `priority:low` - Low priority issue

## 🚨 Security

### Reporting Security Issues
Please do not report security vulnerabilities through public GitHub issues.

Instead, please send an email to [security@yourproject.com] with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Security Best Practices
- Never commit API keys or secrets
- Use environment variables for configuration
- Validate all user inputs
- Implement proper authentication
- Follow OWASP guidelines

## 📞 Getting Help

### Community Support
- **GitHub Discussions** - For questions and general discussion
- **GitHub Issues** - For bug reports and feature requests
- **Documentation** - Check the full documentation first

### Development Questions
- Check existing issues and discussions
- Review the documentation
- Look at similar implementations in the codebase
- Ask specific questions with context

## ⚖️ License

By contributing, you agree that your contributions will be licensed under the MIT License.

## 🎉 Thank You

Thank you for your interest in contributing to the Autonomous Agent! Your contributions help make this project better for everyone.

---

**Happy coding! 🚀**