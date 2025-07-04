import screenshot from 'screenshot-desktop';
import sharp from 'sharp';
import path from 'path';
import fs from 'fs/promises';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export interface CaptureOptions {
  filename?: string;
  format?: 'png' | 'jpg' | 'webp';
  quality?: number;
  screen?: number; // For multi-monitor setups
  crop?: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  resize?: {
    width: number;
    height: number;
  };
}

export interface CaptureResult {
  id: string;
  filepath: string;
  filename: string;
  metadata: {
    timestamp: Date;
    format: string;
    width: number;
    height: number;
    size: number; // file size in bytes
    screen: number;
    processing: {
      cropped: boolean;
      resized: boolean;
      compressed: boolean;
    };
  };
}

export interface ScreenInfo {
  displays: Array<{
    id: number;
    bounds: {
      x: number;
      y: number;
      width: number;
      height: number;
    };
    isPrimary: boolean;
  }>;
}

export class ScreenCaptureService {
  private captureDir: string;
  private isActive: boolean = false;
  private captureQueue: Array<{
    id: string;
    options: CaptureOptions;
    resolve: (result: CaptureResult) => void;
    reject: (error: Error) => void;
  }> = [];
  private processing = false;

  constructor(captureDir?: string) {
    this.captureDir = captureDir || path.join(__dirname, '../../../data/screenshots');
    this.initializeCaptureDirectory();
  }

  private async initializeCaptureDirectory(): Promise<void> {
    try {
      await fs.mkdir(this.captureDir, { recursive: true });
      console.log(`✅ Screenshot directory initialized: ${this.captureDir}`);
      this.isActive = true;
    } catch (error) {
      console.error('❌ Failed to initialize screenshot directory:', error);
      this.isActive = false;
    }
  }

  /**
   * Capture a screenshot with optional processing
   */
  public async captureScreen(options: CaptureOptions = {}): Promise<CaptureResult> {
    if (!this.isActive) {
      throw new Error('Screen capture service is not active');
    }

    return new Promise((resolve, reject) => {
      const id = this.generateCaptureId();
      this.captureQueue.push({ id, options, resolve, reject });
      this.processQueue();
    });
  }

  /**
   * Process the capture queue
   */
  private async processQueue(): Promise<void> {
    if (this.processing || this.captureQueue.length === 0) {
      return;
    }

    this.processing = true;

    while (this.captureQueue.length > 0) {
      const capture = this.captureQueue.shift()!;
      
      try {
        const result = await this.executeCapture(capture.id, capture.options);
        capture.resolve(result);
      } catch (error) {
        capture.reject(error as Error);
      }
    }

    this.processing = false;
  }

  /**
   * Execute the actual screenshot capture and processing
   */
  private async executeCapture(id: string, options: CaptureOptions): Promise<CaptureResult> {
    const timestamp = new Date();
    const format = options.format || 'png';
    const filename = options.filename || `screenshot_${id}.${format}`;
    const filepath = path.join(this.captureDir, filename);

    try {
      // Take the screenshot
      console.log(`📸 Taking screenshot (ID: ${id})`);
      
      const screenshotOptions: any = {
        format: format,
        screen: options.screen || 0
      };

      if (options.quality && format === 'jpg') {
        screenshotOptions.quality = options.quality;
      }

      // Capture the screenshot
      const imageBuffer = await screenshot(screenshotOptions);
      
      // Process the image with Sharp
      let sharpImage = sharp(imageBuffer);
      
      // Get image metadata
      const metadata = await sharpImage.metadata();
      const originalWidth = metadata.width || 0;
      const originalHeight = metadata.height || 0;

      let processing = {
        cropped: false,
        resized: false,
        compressed: false
      };

      // Apply cropping if specified
      if (options.crop) {
        sharpImage = sharpImage.extract({
          left: options.crop.x,
          top: options.crop.y,
          width: options.crop.width,
          height: options.crop.height
        });
        processing.cropped = true;
        console.log(`✂️ Cropping image to ${options.crop.width}x${options.crop.height}`);
      }

      // Apply resizing if specified
      if (options.resize) {
        sharpImage = sharpImage.resize(options.resize.width, options.resize.height, {
          fit: 'inside',
          withoutEnlargement: true
        });
        processing.resized = true;
        console.log(`📏 Resizing image to ${options.resize.width}x${options.resize.height}`);
      }

      // Apply format-specific optimizations
      if (format === 'jpg' || format === 'jpeg') {
        sharpImage = sharpImage.jpeg({ 
          quality: options.quality || 85,
          progressive: true 
        });
        processing.compressed = true;
      } else if (format === 'webp') {
        sharpImage = sharpImage.webp({ 
          quality: options.quality || 85 
        });
        processing.compressed = true;
      } else if (format === 'png') {
        sharpImage = sharpImage.png({ 
          compressionLevel: 6,
          progressive: true 
        });
        processing.compressed = true;
      }

      // Save the processed image
      await sharpImage.toFile(filepath);

      // Get file stats
      const stats = await fs.stat(filepath);
      const processedMetadata = await sharp(filepath).metadata();

      const result: CaptureResult = {
        id,
        filepath,
        filename,
        metadata: {
          timestamp,
          format,
          width: processedMetadata.width || originalWidth,
          height: processedMetadata.height || originalHeight,
          size: stats.size,
          screen: options.screen || 0,
          processing
        }
      };

      console.log(`✅ Screenshot saved: ${filename} (${stats.size} bytes)`);
      return result;

    } catch (error) {
      console.error(`❌ Screenshot capture failed (ID: ${id}):`, error);
      
      // Clean up any partial file
      try {
        await fs.unlink(filepath);
      } catch {
        // File might not exist, ignore
      }
      
      throw new Error(`Screenshot capture failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  /**
   * Capture multiple screenshots at once
   */
  public async captureMultiple(captures: CaptureOptions[]): Promise<CaptureResult[]> {
    const promises = captures.map(options => this.captureScreen(options));
    return Promise.all(promises);
  }

  /**
   * Get screen information
   */
  public async getScreenInfo(): Promise<ScreenInfo> {
    try {
      // Note: screenshot-desktop doesn't provide display info directly
      // This is a simplified implementation - in a real app you might use a different library
      // or platform-specific APIs to get actual display information
      
      return {
        displays: [
          {
            id: 0,
            bounds: {
              x: 0,
              y: 0,
              width: 1920, // Default resolution - should be detected dynamically
              height: 1080
            },
            isPrimary: true
          }
        ]
      };
    } catch (error) {
      console.error('Failed to get screen info:', error);
      throw new Error('Failed to get screen information');
    }
  }

  /**
   * Clean up old screenshots
   */
  public async cleanupOldScreenshots(maxAge: number = 24 * 60 * 60 * 1000): Promise<number> {
    try {
      const files = await fs.readdir(this.captureDir);
      const now = Date.now();
      let deletedCount = 0;

      for (const filename of files) {
        const filepath = path.join(this.captureDir, filename);
        const stats = await fs.stat(filepath);
        
        if (now - stats.mtime.getTime() > maxAge) {
          await fs.unlink(filepath);
          deletedCount++;
          console.log(`🗑️ Deleted old screenshot: ${filename}`);
        }
      }

      console.log(`✅ Cleanup complete: ${deletedCount} files deleted`);
      return deletedCount;
    } catch (error) {
      console.error('Screenshot cleanup failed:', error);
      return 0;
    }
  }

  /**
   * Get statistics about stored screenshots
   */
  public async getStorageStats(): Promise<{
    fileCount: number;
    totalSize: number;
    averageSize: number;
    oldestFile: Date | null;
    newestFile: Date | null;
  }> {
    try {
      const files = await fs.readdir(this.captureDir);
      let totalSize = 0;
      let oldestTime = Infinity;
      let newestTime = 0;

      for (const filename of files) {
        const filepath = path.join(this.captureDir, filename);
        const stats = await fs.stat(filepath);
        
        totalSize += stats.size;
        oldestTime = Math.min(oldestTime, stats.mtime.getTime());
        newestTime = Math.max(newestTime, stats.mtime.getTime());
      }

      return {
        fileCount: files.length,
        totalSize,
        averageSize: files.length > 0 ? totalSize / files.length : 0,
        oldestFile: oldestTime !== Infinity ? new Date(oldestTime) : null,
        newestFile: newestTime > 0 ? new Date(newestTime) : null
      };
    } catch (error) {
      console.error('Failed to get storage stats:', error);
      return {
        fileCount: 0,
        totalSize: 0,
        averageSize: 0,
        oldestFile: null,
        newestFile: null
      };
    }
  }

  /**
   * Compare two screenshots for differences
   */
  public async compareScreenshots(
    filepath1: string, 
    filepath2: string, 
    threshold: number = 0.1
  ): Promise<{
    isDifferent: boolean;
    differencePercentage: number;
    diffImagePath?: string;
  }> {
    try {
      // Load both images
      const image1 = sharp(filepath1);
      const image2 = sharp(filepath2);

      // Get metadata to ensure same dimensions
      const meta1 = await image1.metadata();
      const meta2 = await image2.metadata();

      if (meta1.width !== meta2.width || meta1.height !== meta2.height) {
        throw new Error('Images must have the same dimensions for comparison');
      }

      // Convert to raw pixel data
      const data1 = await image1.raw().toBuffer();
      const data2 = await image2.raw().toBuffer();

      // Calculate pixel differences
      let differentPixels = 0;
      const totalPixels = data1.length / 3; // RGB channels

      for (let i = 0; i < data1.length; i += 3) {
        const r1 = data1[i], g1 = data1[i + 1], b1 = data1[i + 2];
        const r2 = data2[i], g2 = data2[i + 1], b2 = data2[i + 2];
        
        // Calculate color difference using simple Euclidean distance
        const diff = Math.sqrt(
          Math.pow(r1 - r2, 2) + 
          Math.pow(g1 - g2, 2) + 
          Math.pow(b1 - b2, 2)
        ) / (255 * Math.sqrt(3)); // Normalize to 0-1

        if (diff > threshold) {
          differentPixels++;
        }
      }

      const differencePercentage = (differentPixels / totalPixels) * 100;
      const isDifferent = differencePercentage > (threshold * 100);

      return {
        isDifferent,
        differencePercentage
      };

    } catch (error) {
      console.error('Screenshot comparison failed:', error);
      throw new Error(`Failed to compare screenshots: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  /**
   * Take a screenshot of a specific window (requires additional setup)
   */
  public async captureWindow(windowTitle: string, options: CaptureOptions = {}): Promise<CaptureResult> {
    // This would require platform-specific implementations
    // For now, fall back to full screen capture
    console.warn('Window-specific capture not implemented, falling back to full screen');
    return this.captureScreen(options);
  }

  // Utility methods
  private generateCaptureId(): string {
    return `cap_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  public isActive(): boolean {
    return this.isActive;
  }

  public getQueueLength(): number {
    return this.captureQueue.length;
  }

  public getCaptureDirectory(): string {
    return this.captureDir;
  }

  public async setCaptureDirectory(newDir: string): Promise<void> {
    this.captureDir = newDir;
    await this.initializeCaptureDirectory();
  }
}