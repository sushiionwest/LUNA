import sqlite3 from 'sqlite3';
import { promisify } from 'util';
import path from 'path';
import fs from 'fs/promises';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export interface Task {
  id: string;
  name: string;
  description: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  type: 'screen_capture' | 'social_media' | 'automation' | 'installation' | 'custom';
  priority: number;
  parameters: string; // JSON string
  result?: string; // JSON string
  error?: string;
  startTime?: Date;
  endTime?: Date;
  duration?: number; // milliseconds
  retryCount: number;
  maxRetries: number;
  createdAt: Date;
  updatedAt: Date;
}

export interface ActivityLog {
  id: string;
  taskId?: string;
  type: 'info' | 'warning' | 'error' | 'success';
  category: 'system' | 'agent' | 'user' | 'api';
  message: string;
  details?: string; // JSON string
  timestamp: Date;
  source: string;
}

export interface ScreenCapture {
  id: string;
  taskId?: string;
  imagePath: string;
  metadata: string; // JSON string with dimensions, timestamp, etc.
  analysis?: string; // JSON string with AI analysis results
  timestamp: Date;
}

export interface SocialMediaPost {
  id: string;
  platform: 'twitter' | 'instagram' | 'linkedin';
  postId?: string;
  content: string;
  mediaUrls?: string; // JSON array
  status: 'draft' | 'scheduled' | 'posted' | 'failed';
  scheduledFor?: Date;
  postedAt?: Date;
  engagementData?: string; // JSON string
  createdAt: Date;
  updatedAt: Date;
}

export interface SystemMetric {
  id: string;
  metric: string;
  value: number;
  unit: string;
  timestamp: Date;
  category: 'performance' | 'usage' | 'error' | 'success';
}

export class DatabaseService {
  private db: sqlite3.Database | null = null;
  private dbPath: string;
  private isConnected: boolean = false;

  constructor(dbPath?: string) {
    this.dbPath = dbPath || path.join(__dirname, '../../../data/agent.db');
  }

  public async initialize(): Promise<void> {
    try {
      // Ensure data directory exists
      const dataDir = path.dirname(this.dbPath);
      await fs.mkdir(dataDir, { recursive: true });

      // Open database connection
      this.db = new sqlite3.Database(this.dbPath);
      
      // Promisify database methods
      const dbRun = promisify(this.db.run.bind(this.db));
      const dbGet = promisify(this.db.get.bind(this.db));
      const dbAll = promisify(this.db.all.bind(this.db));

      // Create tables
      await this.createTables();
      
      this.isConnected = true;
      console.log(`✅ Database initialized at ${this.dbPath}`);
    } catch (error) {
      console.error('❌ Database initialization failed:', error);
      throw error;
    }
  }

  private async createTables(): Promise<void> {
    if (!this.db) throw new Error('Database not initialized');

    const dbRun = promisify(this.db.run.bind(this.db));

    // Tasks table
    await dbRun(`
      CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
        type TEXT NOT NULL CHECK (type IN ('screen_capture', 'social_media', 'automation', 'installation', 'custom')),
        priority INTEGER DEFAULT 1,
        parameters TEXT,
        result TEXT,
        error TEXT,
        start_time DATETIME,
        end_time DATETIME,
        duration INTEGER,
        retry_count INTEGER DEFAULT 0,
        max_retries INTEGER DEFAULT 3,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
      )
    `);

    // Activity logs table
    await dbRun(`
      CREATE TABLE IF NOT EXISTS activity_logs (
        id TEXT PRIMARY KEY,
        task_id TEXT,
        type TEXT NOT NULL CHECK (type IN ('info', 'warning', 'error', 'success')),
        category TEXT NOT NULL CHECK (category IN ('system', 'agent', 'user', 'api')),
        message TEXT NOT NULL,
        details TEXT,
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
        source TEXT NOT NULL,
        FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
      )
    `);

    // Screen captures table
    await dbRun(`
      CREATE TABLE IF NOT EXISTS screen_captures (
        id TEXT PRIMARY KEY,
        task_id TEXT,
        image_path TEXT NOT NULL,
        metadata TEXT,
        analysis TEXT,
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
      )
    `);

    // Social media posts table
    await dbRun(`
      CREATE TABLE IF NOT EXISTS social_media_posts (
        id TEXT PRIMARY KEY,
        platform TEXT NOT NULL CHECK (platform IN ('twitter', 'instagram', 'linkedin')),
        post_id TEXT,
        content TEXT NOT NULL,
        media_urls TEXT,
        status TEXT NOT NULL CHECK (status IN ('draft', 'scheduled', 'posted', 'failed')),
        scheduled_for DATETIME,
        posted_at DATETIME,
        engagement_data TEXT,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
      )
    `);

    // System metrics table
    await dbRun(`
      CREATE TABLE IF NOT EXISTS system_metrics (
        id TEXT PRIMARY KEY,
        metric TEXT NOT NULL,
        value REAL NOT NULL,
        unit TEXT NOT NULL,
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
        category TEXT NOT NULL CHECK (category IN ('performance', 'usage', 'error', 'success'))
      )
    `);

    // Create indexes for better performance
    await dbRun('CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status)');
    await dbRun('CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks (created_at)');
    await dbRun('CREATE INDEX IF NOT EXISTS idx_activity_logs_timestamp ON activity_logs (timestamp)');
    await dbRun('CREATE INDEX IF NOT EXISTS idx_activity_logs_category ON activity_logs (category)');
    await dbRun('CREATE INDEX IF NOT EXISTS idx_screen_captures_timestamp ON screen_captures (timestamp)');
    await dbRun('CREATE INDEX IF NOT EXISTS idx_social_media_posts_platform ON social_media_posts (platform)');
    await dbRun('CREATE INDEX IF NOT EXISTS idx_system_metrics_timestamp ON system_metrics (timestamp)');
  }

  // Task management methods
  public async createTask(task: Omit<Task, 'id' | 'createdAt' | 'updatedAt'>): Promise<string> {
    if (!this.db) throw new Error('Database not initialized');

    const dbRun = promisify(this.db.run.bind(this.db));
    const id = this.generateId();

    await dbRun(`
      INSERT INTO tasks (
        id, name, description, status, type, priority, parameters, 
        result, error, start_time, end_time, duration, retry_count, max_retries
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `, [
      id, task.name, task.description, task.status, task.type, task.priority,
      task.parameters, task.result, task.error, task.startTime, task.endTime,
      task.duration, task.retryCount, task.maxRetries
    ]);

    return id;
  }

  public async updateTask(id: string, updates: Partial<Task>): Promise<void> {
    if (!this.db) throw new Error('Database not initialized');

    const dbRun = promisify(this.db.run.bind(this.db));
    const fields = Object.keys(updates).filter(key => key !== 'id').map(key => {
      // Convert camelCase to snake_case for database columns
      return key.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);
    });
    const values = Object.values(updates).filter((_, index) => Object.keys(updates)[index] !== 'id');

    if (fields.length === 0) return;

    const setClause = fields.map(field => `${field} = ?`).join(', ');
    await dbRun(`UPDATE tasks SET ${setClause}, updated_at = CURRENT_TIMESTAMP WHERE id = ?`, [...values, id]);
  }

  public async getTask(id: string): Promise<Task | null> {
    if (!this.db) throw new Error('Database not initialized');

    const dbGet = promisify(this.db.get.bind(this.db));
    const row = await dbGet('SELECT * FROM tasks WHERE id = ?', [id]);
    return row ? this.mapRowToTask(row) : null;
  }

  public async getTasks(options: {
    status?: string;
    type?: string;
    limit?: number;
    offset?: number;
    orderBy?: string;
    orderDirection?: 'ASC' | 'DESC';
  } = {}): Promise<Task[]> {
    if (!this.db) throw new Error('Database not initialized');

    const dbAll = promisify(this.db.all.bind(this.db));
    
    let query = 'SELECT * FROM tasks WHERE 1=1';
    const params: any[] = [];

    if (options.status) {
      query += ' AND status = ?';
      params.push(options.status);
    }

    if (options.type) {
      query += ' AND type = ?';
      params.push(options.type);
    }

    const orderBy = options.orderBy || 'created_at';
    const orderDirection = options.orderDirection || 'DESC';
    query += ` ORDER BY ${orderBy} ${orderDirection}`;

    if (options.limit) {
      query += ' LIMIT ?';
      params.push(options.limit);
    }

    if (options.offset) {
      query += ' OFFSET ?';
      params.push(options.offset);
    }

    const rows = await dbAll(query, params);
    return rows.map(row => this.mapRowToTask(row));
  }

  // Activity log methods
  public async logActivity(log: Omit<ActivityLog, 'id' | 'timestamp'>): Promise<string> {
    if (!this.db) throw new Error('Database not initialized');

    const dbRun = promisify(this.db.run.bind(this.db));
    const id = this.generateId();

    await dbRun(`
      INSERT INTO activity_logs (id, task_id, type, category, message, details, source)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `, [id, log.taskId, log.type, log.category, log.message, log.details, log.source]);

    return id;
  }

  public async getActivityLogs(options: {
    taskId?: string;
    type?: string;
    category?: string;
    limit?: number;
    offset?: number;
  } = {}): Promise<ActivityLog[]> {
    if (!this.db) throw new Error('Database not initialized');

    const dbAll = promisify(this.db.all.bind(this.db));
    
    let query = 'SELECT * FROM activity_logs WHERE 1=1';
    const params: any[] = [];

    if (options.taskId) {
      query += ' AND task_id = ?';
      params.push(options.taskId);
    }

    if (options.type) {
      query += ' AND type = ?';
      params.push(options.type);
    }

    if (options.category) {
      query += ' AND category = ?';
      params.push(options.category);
    }

    query += ' ORDER BY timestamp DESC';

    if (options.limit) {
      query += ' LIMIT ?';
      params.push(options.limit);
    }

    if (options.offset) {
      query += ' OFFSET ?';
      params.push(options.offset);
    }

    const rows = await dbAll(query, params);
    return rows.map(row => this.mapRowToActivityLog(row));
  }

  // Screen capture methods
  public async saveScreenCapture(capture: Omit<ScreenCapture, 'id' | 'timestamp'>): Promise<string> {
    if (!this.db) throw new Error('Database not initialized');

    const dbRun = promisify(this.db.run.bind(this.db));
    const id = this.generateId();

    await dbRun(`
      INSERT INTO screen_captures (id, task_id, image_path, metadata, analysis)
      VALUES (?, ?, ?, ?, ?)
    `, [id, capture.taskId, capture.imagePath, capture.metadata, capture.analysis]);

    return id;
  }

  // Social media post methods
  public async saveSocialMediaPost(post: Omit<SocialMediaPost, 'id' | 'createdAt' | 'updatedAt'>): Promise<string> {
    if (!this.db) throw new Error('Database not initialized');

    const dbRun = promisify(this.db.run.bind(this.db));
    const id = this.generateId();

    await dbRun(`
      INSERT INTO social_media_posts (
        id, platform, post_id, content, media_urls, status, 
        scheduled_for, posted_at, engagement_data
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `, [
      id, post.platform, post.postId, post.content, post.mediaUrls,
      post.status, post.scheduledFor, post.postedAt, post.engagementData
    ]);

    return id;
  }

  // System metrics methods
  public async recordMetric(metric: Omit<SystemMetric, 'id' | 'timestamp'>): Promise<string> {
    if (!this.db) throw new Error('Database not initialized');

    const dbRun = promisify(this.db.run.bind(this.db));
    const id = this.generateId();

    await dbRun(`
      INSERT INTO system_metrics (id, metric, value, unit, category)
      VALUES (?, ?, ?, ?, ?)
    `, [id, metric.metric, metric.value, metric.unit, metric.category]);

    return id;
  }

  public async getMetrics(options: {
    metric?: string;
    category?: string;
    startTime?: Date;
    endTime?: Date;
    limit?: number;
  } = {}): Promise<SystemMetric[]> {
    if (!this.db) throw new Error('Database not initialized');

    const dbAll = promisify(this.db.all.bind(this.db));
    
    let query = 'SELECT * FROM system_metrics WHERE 1=1';
    const params: any[] = [];

    if (options.metric) {
      query += ' AND metric = ?';
      params.push(options.metric);
    }

    if (options.category) {
      query += ' AND category = ?';
      params.push(options.category);
    }

    if (options.startTime) {
      query += ' AND timestamp >= ?';
      params.push(options.startTime.toISOString());
    }

    if (options.endTime) {
      query += ' AND timestamp <= ?';
      params.push(options.endTime.toISOString());
    }

    query += ' ORDER BY timestamp DESC';

    if (options.limit) {
      query += ' LIMIT ?';
      params.push(options.limit);
    }

    const rows = await dbAll(query, params);
    return rows.map(row => this.mapRowToSystemMetric(row));
  }

  // Utility methods
  private generateId(): string {
    return Date.now().toString(36) + Math.random().toString(36).substr(2);
  }

  private mapRowToTask(row: any): Task {
    return {
      id: row.id,
      name: row.name,
      description: row.description,
      status: row.status,
      type: row.type,
      priority: row.priority,
      parameters: row.parameters,
      result: row.result,
      error: row.error,
      startTime: row.start_time ? new Date(row.start_time) : undefined,
      endTime: row.end_time ? new Date(row.end_time) : undefined,
      duration: row.duration,
      retryCount: row.retry_count,
      maxRetries: row.max_retries,
      createdAt: new Date(row.created_at),
      updatedAt: new Date(row.updated_at),
    };
  }

  private mapRowToActivityLog(row: any): ActivityLog {
    return {
      id: row.id,
      taskId: row.task_id,
      type: row.type,
      category: row.category,
      message: row.message,
      details: row.details,
      timestamp: new Date(row.timestamp),
      source: row.source,
    };
  }

  private mapRowToSystemMetric(row: any): SystemMetric {
    return {
      id: row.id,
      metric: row.metric,
      value: row.value,
      unit: row.unit,
      timestamp: new Date(row.timestamp),
      category: row.category,
    };
  }

  public isConnected(): boolean {
    return this.isConnected;
  }

  public async close(): Promise<void> {
    if (this.db) {
      const dbClose = promisify(this.db.close.bind(this.db));
      await dbClose();
      this.isConnected = false;
      console.log('✅ Database connection closed');
    }
  }
}