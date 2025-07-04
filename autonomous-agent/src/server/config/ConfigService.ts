import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Load environment variables
dotenv.config();

export interface DatabaseConfig {
  type: 'sqlite' | 'postgresql' | 'mysql';
  path?: string;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  password?: string;
}

export interface MicrosoftVisionConfig {
  subscriptionKey: string;
  endpoint: string;
  region: string;
}

export interface TwitterConfig {
  apiKey: string;
  apiSecretKey: string;
  accessToken: string;
  accessTokenSecret: string;
  bearerToken: string;
}

export interface InstagramConfig {
  accessToken: string;
  clientId: string;
  clientSecret: string;
}

export interface LinkedInConfig {
  clientId: string;
  clientSecret: string;
  accessToken: string;
}

export interface SocialMediaConfig {
  twitter: TwitterConfig;
  instagram: InstagramConfig;
  linkedin: LinkedInConfig;
}

export interface AgentConfig {
  maxConcurrentTasks: number;
  taskTimeout: number;
  retryAttempts: number;
  screenCaptureInterval: number;
  autoRetry: boolean;
  safeMode: boolean;
}

export interface SecurityConfig {
  jwtSecret: string;
  bcryptRounds: number;
  rateLimit: {
    windowMs: number;
    maxRequests: number;
  };
  allowedOrigins: string[];
}

export class ConfigService {
  private config: {
    database: DatabaseConfig;
    microsoftVision: MicrosoftVisionConfig;
    socialMedia: SocialMediaConfig;
    agent: AgentConfig;
    security: SecurityConfig;
    server: {
      port: number;
      environment: string;
      corsOrigin: string;
    };
  };

  constructor() {
    this.config = this.loadConfiguration();
    this.validateConfiguration();
  }

  private loadConfiguration() {
    return {
      database: {
        type: (process.env.DB_TYPE as 'sqlite' | 'postgresql' | 'mysql') || 'sqlite',
        path: process.env.DB_PATH || path.join(__dirname, '../../data/agent.db'),
        host: process.env.DB_HOST,
        port: process.env.DB_PORT ? parseInt(process.env.DB_PORT) : undefined,
        database: process.env.DB_NAME,
        username: process.env.DB_USERNAME,
        password: process.env.DB_PASSWORD,
      },
      microsoftVision: {
        subscriptionKey: process.env.MICROSOFT_VISION_KEY || '',
        endpoint: process.env.MICROSOFT_VISION_ENDPOINT || '',
        region: process.env.MICROSOFT_VISION_REGION || 'eastus',
      },
      socialMedia: {
        twitter: {
          apiKey: process.env.TWITTER_API_KEY || '',
          apiSecretKey: process.env.TWITTER_API_SECRET || '',
          accessToken: process.env.TWITTER_ACCESS_TOKEN || '',
          accessTokenSecret: process.env.TWITTER_ACCESS_TOKEN_SECRET || '',
          bearerToken: process.env.TWITTER_BEARER_TOKEN || '',
        },
        instagram: {
          accessToken: process.env.INSTAGRAM_ACCESS_TOKEN || '',
          clientId: process.env.INSTAGRAM_CLIENT_ID || '',
          clientSecret: process.env.INSTAGRAM_CLIENT_SECRET || '',
        },
        linkedin: {
          clientId: process.env.LINKEDIN_CLIENT_ID || '',
          clientSecret: process.env.LINKEDIN_CLIENT_SECRET || '',
          accessToken: process.env.LINKEDIN_ACCESS_TOKEN || '',
        },
      },
      agent: {
        maxConcurrentTasks: parseInt(process.env.AGENT_MAX_CONCURRENT_TASKS || '3'),
        taskTimeout: parseInt(process.env.AGENT_TASK_TIMEOUT || '300000'), // 5 minutes
        retryAttempts: parseInt(process.env.AGENT_RETRY_ATTEMPTS || '3'),
        screenCaptureInterval: parseInt(process.env.SCREEN_CAPTURE_INTERVAL || '1000'), // 1 second
        autoRetry: process.env.AGENT_AUTO_RETRY === 'true',
        safeMode: process.env.AGENT_SAFE_MODE !== 'false', // Default to true
      },
      security: {
        jwtSecret: process.env.JWT_SECRET || 'your-super-secret-jwt-key-change-this-in-production',
        bcryptRounds: parseInt(process.env.BCRYPT_ROUNDS || '12'),
        rateLimit: {
          windowMs: parseInt(process.env.RATE_LIMIT_WINDOW_MS || '900000'), // 15 minutes
          maxRequests: parseInt(process.env.RATE_LIMIT_MAX_REQUESTS || '100'),
        },
        allowedOrigins: process.env.ALLOWED_ORIGINS?.split(',') || ['*'],
      },
      server: {
        port: parseInt(process.env.PORT || '3001'),
        environment: process.env.NODE_ENV || 'development',
        corsOrigin: process.env.CORS_ORIGIN || '*',
      },
    };
  }

  private validateConfiguration(): void {
    const errors: string[] = [];

    // Validate database configuration
    if (this.config.database.type === 'sqlite' && !this.config.database.path) {
      errors.push('SQLite database path is required');
    }

    // Validate Microsoft Vision API (optional for basic functionality)
    if (!this.config.microsoftVision.subscriptionKey && process.env.NODE_ENV === 'production') {
      console.warn('⚠️ Microsoft Vision API key not configured - vision features will be disabled');
    }

    // Validate social media APIs (optional)
    if (!this.config.socialMedia.twitter.apiKey && process.env.NODE_ENV === 'production') {
      console.warn('⚠️ Twitter API credentials not configured - Twitter features will be disabled');
    }

    // Validate security settings
    if (this.config.security.jwtSecret === 'your-super-secret-jwt-key-change-this-in-production') {
      if (process.env.NODE_ENV === 'production') {
        errors.push('JWT secret must be changed in production');
      } else {
        console.warn('⚠️ Using default JWT secret - change this in production');
      }
    }

    if (errors.length > 0) {
      console.error('❌ Configuration validation failed:');
      errors.forEach(error => console.error(`  - ${error}`));
      throw new Error('Invalid configuration');
    }

    console.log('✅ Configuration validated successfully');
  }

  // Getters for different configuration sections
  public getDatabaseConfig(): DatabaseConfig {
    return { ...this.config.database };
  }

  public getMicrosoftVisionConfig(): MicrosoftVisionConfig {
    return { ...this.config.microsoftVision };
  }

  public getSocialMediaConfig(): SocialMediaConfig {
    return { ...this.config.socialMedia };
  }

  public getAgentConfig(): AgentConfig {
    return { ...this.config.agent };
  }

  public getSecurityConfig(): SecurityConfig {
    return { ...this.config.security };
  }

  public getServerConfig() {
    return { ...this.config.server };
  }

  // Utility methods
  public isProduction(): boolean {
    return this.config.server.environment === 'production';
  }

  public isDevelopment(): boolean {
    return this.config.server.environment === 'development';
  }

  public isMicrosoftVisionEnabled(): boolean {
    return !!(this.config.microsoftVision.subscriptionKey && this.config.microsoftVision.endpoint);
  }

  public isTwitterEnabled(): boolean {
    return !!(this.config.socialMedia.twitter.apiKey && this.config.socialMedia.twitter.apiSecretKey);
  }

  public isInstagramEnabled(): boolean {
    return !!(this.config.socialMedia.instagram.accessToken && this.config.socialMedia.instagram.clientId);
  }

  public isLinkedInEnabled(): boolean {
    return !!(this.config.socialMedia.linkedin.clientId && this.config.socialMedia.linkedin.accessToken);
  }

  // Method to update configuration at runtime (for settings UI)
  public updateConfiguration(section: string, updates: any): void {
    if (section in this.config) {
      this.config[section as keyof typeof this.config] = {
        ...this.config[section as keyof typeof this.config],
        ...updates,
      };
      
      // Re-validate after updates
      this.validateConfiguration();
      
      console.log(`📝 Configuration section '${section}' updated`);
    } else {
      throw new Error(`Invalid configuration section: ${section}`);
    }
  }

  // Method to get masked configuration (for frontend display)
  public getMaskedConfiguration() {
    return {
      database: {
        type: this.config.database.type,
        connected: true, // This would be determined by DatabaseService
      },
      microsoftVision: {
        configured: this.isMicrosoftVisionEnabled(),
        region: this.config.microsoftVision.region,
      },
      socialMedia: {
        twitter: { configured: this.isTwitterEnabled() },
        instagram: { configured: this.isInstagramEnabled() },
        linkedin: { configured: this.isLinkedInEnabled() },
      },
      agent: this.config.agent,
      server: {
        environment: this.config.server.environment,
        port: this.config.server.port,
      },
    };
  }
}