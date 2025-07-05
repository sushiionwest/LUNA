import { EventEmitter } from 'events';
import * as net from 'net';
import { v4 as uuidv4 } from 'uuid';
import crypto from 'crypto';

export interface BrokerRequest {
  requestId: string;
  operation: string;
  parameters: any;
  timestamp: Date;
  signature?: string;
}

export interface BrokerResponse {
  success: boolean;
  data?: any;
  errorMessage?: string;
  timestamp: Date;
}

export interface UIAutomationClickParams {
  x: number;
  y: number;
  button?: 'left' | 'right' | 'middle';
}

export interface UIAutomationSendKeysParams {
  keys: string;
  targetWindow?: string;
}

export interface RegistryParams {
  keyPath: string;
  valueName: string;
  value?: any;
}

export interface ProcessParams {
  fileName: string;
  arguments?: string;
  processId?: number;
  workingDirectory?: string;
}

export interface FileParams {
  filePath: string;
  content?: string;
  encoding?: string;
}

export interface WindowInfo {
  handle: string;
  title: string;
  className: string;
  processId: number;
  processName: string;
  isVisible: boolean;
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

export class PrivilegeBrokerClient extends EventEmitter {
  private readonly pipeName = '\\\\.\\pipe\\LunaBrokerService';
  private client: net.Socket | null = null;
  private isConnected = false;
  private requestQueue = new Map<string, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
    timeout: NodeJS.Timeout;
  }>();
  private connectionRetryCount = 0;
  private readonly maxRetries = 5;
  private readonly retryDelay = 1000; // 1 second
  private secretKey: string;

  constructor() {
    super();
    this.secretKey = this.loadOrCreateSecretKey();
  }

  private loadOrCreateSecretKey(): string {
    // In a real implementation, this would load from secure storage
    // For now, using a hardcoded key (should be replaced with proper key management)
    return process.env.LUNA_BROKER_SECRET || 'default-secret-key-replace-in-production';
  }

  public async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.isConnected) {
        resolve();
        return;
      }

      this.client = net.createConnection(this.pipeName);

      this.client.on('connect', () => {
        console.log('Connected to Luna Broker Service');
        this.isConnected = true;
        this.connectionRetryCount = 0;
        this.emit('connected');
        resolve();
      });

      this.client.on('data', (data) => {
        this.handleResponse(data.toString());
      });

      this.client.on('error', (error) => {
        console.error('Broker client error:', error);
        this.isConnected = false;
        this.emit('error', error);
        
        if (this.connectionRetryCount < this.maxRetries) {
          this.connectionRetryCount++;
          console.log(`Retrying connection (${this.connectionRetryCount}/${this.maxRetries})...`);
          setTimeout(() => {
            this.connect().catch(() => {
              // Retry failed
            });
          }, this.retryDelay * this.connectionRetryCount);
        } else {
          reject(new Error('Failed to connect to broker service after maximum retries'));
        }
      });

      this.client.on('close', () => {
        console.log('Disconnected from Luna Broker Service');
        this.isConnected = false;
        this.emit('disconnected');
        
        // Clear pending requests
        this.requestQueue.forEach(({ reject, timeout }) => {
          clearTimeout(timeout);
          reject(new Error('Connection closed'));
        });
        this.requestQueue.clear();
      });

      // Set connection timeout
      setTimeout(() => {
        if (!this.isConnected) {
          this.client?.destroy();
          reject(new Error('Connection timeout'));
        }
      }, 10000); // 10 seconds
    });
  }

  public disconnect(): void {
    if (this.client) {
      this.client.end();
      this.client = null;
    }
    this.isConnected = false;
  }

  private handleResponse(responseData: string): void {
    try {
      const lines = responseData.trim().split('\n');
      
      for (const line of lines) {
        if (!line.trim()) continue;
        
        const response: BrokerResponse = JSON.parse(line);
        
        // For now, we'll match responses to requests based on timestamp
        // In a real implementation, you'd want request IDs in responses
        const oldestRequest = Array.from(this.requestQueue.entries())
          .sort(([, a], [, b]) => 0)[0]; // Get first request
        
        if (oldestRequest) {
          const [requestId, { resolve, reject, timeout }] = oldestRequest;
          this.requestQueue.delete(requestId);
          clearTimeout(timeout);
          
          if (response.success) {
            resolve(response.data);
          } else {
            reject(new Error(response.errorMessage || 'Unknown error'));
          }
        }
      }
    } catch (error) {
      console.error('Error parsing broker response:', error);
    }
  }

  private async sendRequest<T>(operation: string, parameters?: any): Promise<T> {
    if (!this.isConnected) {
      await this.connect();
    }

    return new Promise((resolve, reject) => {
      const requestId = uuidv4();
      const request: BrokerRequest = {
        requestId,
        operation,
        parameters: parameters || {},
        timestamp: new Date()
      };

      // Sign the request
      request.signature = this.signRequest(request);

      const timeout = setTimeout(() => {
        this.requestQueue.delete(requestId);
        reject(new Error('Request timeout'));
      }, 30000); // 30 seconds

      this.requestQueue.set(requestId, { resolve, reject, timeout });

      const requestJson = JSON.stringify(request);
      this.client?.write(requestJson + '\n');
    });
  }

  private signRequest(request: BrokerRequest): string {
    const payload = `${request.requestId}:${request.operation}:${JSON.stringify(request.parameters)}:${request.timestamp.toISOString()}`;
    const hmac = crypto.createHmac('sha256', this.secretKey);
    hmac.update(payload);
    return hmac.digest('hex');
  }

  // UI Automation Methods
  public async click(x: number, y: number, button: 'left' | 'right' | 'middle' = 'left'): Promise<{ success: boolean; x: number; y: number }> {
    const params: UIAutomationClickParams = { x, y, button };
    return this.sendRequest<{ success: boolean; x: number; y: number }>('uiautomation.click', params);
  }

  public async sendKeys(keys: string, targetWindow?: string): Promise<{ success: boolean; keys: string }> {
    const params: UIAutomationSendKeysParams = { keys, targetWindow };
    return this.sendRequest<{ success: boolean; keys: string }>('uiautomation.sendkeys', params);
  }

  public async getWindows(): Promise<{ windows: WindowInfo[] }> {
    return this.sendRequest<{ windows: WindowInfo[] }>('uiautomation.getwindows');
  }

  // Registry Methods
  public async readRegistry(keyPath: string, valueName: string): Promise<{ value: any }> {
    const params: RegistryParams = { keyPath, valueName };
    return this.sendRequest<{ value: any }>('registry.read', params);
  }

  public async writeRegistry(keyPath: string, valueName: string, value: any): Promise<{ success: boolean }> {
    const params: RegistryParams = { keyPath, valueName, value };
    return this.sendRequest<{ success: boolean }>('registry.write', params);
  }

  // Process Methods
  public async startProcess(fileName: string, arguments?: string, workingDirectory?: string): Promise<{ processId: number }> {
    const params: ProcessParams = { fileName, arguments, workingDirectory };
    return this.sendRequest<{ processId: number }>('process.start', params);
  }

  public async terminateProcess(processId: number): Promise<{ success: boolean }> {
    const params: ProcessParams = { fileName: '', processId };
    return this.sendRequest<{ success: boolean }>('process.terminate', params);
  }

  // File Methods
  public async readFile(filePath: string, encoding: string = 'utf-8'): Promise<{ content: string }> {
    const params: FileParams = { filePath, encoding };
    return this.sendRequest<{ content: string }>('file.read', params);
  }

  public async writeFile(filePath: string, content: string, encoding: string = 'utf-8'): Promise<{ success: boolean }> {
    const params: FileParams = { filePath, content, encoding };
    return this.sendRequest<{ success: boolean }>('file.write', params);
  }

  // System Methods
  public async takeScreenshot(): Promise<{ screenshotPath: string }> {
    return this.sendRequest<{ screenshotPath: string }>('system.screenshot');
  }

  // Utility Methods
  public isConnectedToBroker(): boolean {
    return this.isConnected;
  }

  public getConnectionStatus(): {
    connected: boolean;
    retryCount: number;
    maxRetries: number;
  } {
    return {
      connected: this.isConnected,
      retryCount: this.connectionRetryCount,
      maxRetries: this.maxRetries
    };
  }

  // Security Methods
  public async testConnection(): Promise<boolean> {
    try {
      await this.getWindows();
      return true;
    } catch (error) {
      console.error('Broker connection test failed:', error);
      return false;
    }
  }

  // Check if broker service is running
  public static async isBrokerServiceRunning(): Promise<boolean> {
    try {
      const { exec } = require('child_process');
      const { promisify } = require('util');
      const execAsync = promisify(exec);
      
      const { stdout } = await execAsync('sc query "LunaBrokerService"');
      return stdout.includes('RUNNING');
    } catch (error) {
      return false;
    }
  }

  // Start broker service if not running
  public static async startBrokerService(): Promise<boolean> {
    try {
      const { exec } = require('child_process');
      const { promisify } = require('util');
      const execAsync = promisify(exec);
      
      await execAsync('net start "LunaBrokerService"');
      return true;
    } catch (error) {
      console.error('Failed to start broker service:', error);
      return false;
    }
  }
}