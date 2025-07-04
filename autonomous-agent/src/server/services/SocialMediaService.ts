import { ComputerVisionClient } from '@azure/cognitiveservices-computervision';
import { ApiKeyCredentials } from '@azure/ms-rest-js';
import { TwitterApi } from 'twitter-api-v2';
import axios from 'axios';
import fs from 'fs/promises';
import path from 'path';
import { ConfigService } from '../config/ConfigService.js';

export interface VisionAnalysisResult {
  description: string;
  tags: string[];
  objects: Array<{
    name: string;
    confidence: number;
    boundingBox: {
      x: number;
      y: number;
      width: number;
      height: number;
    };
  }>;
  text: string[];
  faces: Array<{
    age: number;
    gender: string;
    emotion: string;
    confidence: number;
  }>;
  categories: Array<{
    name: string;
    confidence: number;
  }>;
  color: {
    dominantColors: string[];
    accentColor: string;
    isBWImg: boolean;
  };
  imageType: {
    clipArtType: number;
    lineDrawingType: number;
  };
}

export interface SocialMediaPost {
  platform: 'twitter' | 'instagram' | 'linkedin';
  content: string;
  mediaUrls?: string[];
  scheduledFor?: Date;
  hashtags?: string[];
  mentions?: string[];
}

export interface PostResult {
  success: boolean;
  postId?: string;
  url?: string;
  error?: string;
  engagementData?: {
    likes: number;
    retweets: number;
    replies: number;
    views: number;
  };
}

export interface ContentGenerationOptions {
  topic: string;
  tone: 'professional' | 'casual' | 'humorous' | 'informative';
  length: 'short' | 'medium' | 'long';
  includeHashtags: boolean;
  platform: 'twitter' | 'instagram' | 'linkedin';
  imageAnalysis?: VisionAnalysisResult;
}

export class SocialMediaService {
  private visionClient: ComputerVisionClient | null = null;
  private twitterClient: TwitterApi | null = null;
  private configService: ConfigService;

  constructor(configService: ConfigService) {
    this.configService = configService;
    this.initializeClients();
  }

  private initializeClients(): void {
    try {
      // Initialize Microsoft Vision API client
      const visionConfig = this.configService.getMicrosoftVisionConfig();
      if (visionConfig.subscriptionKey && visionConfig.endpoint) {
        const credentials = new ApiKeyCredentials({
          inHeader: { 'Ocp-Apim-Subscription-Key': visionConfig.subscriptionKey }
        });
        this.visionClient = new ComputerVisionClient(credentials, visionConfig.endpoint);
        console.log('✅ Microsoft Vision API client initialized');
      } else {
        console.warn('⚠️ Microsoft Vision API not configured');
      }

      // Initialize Twitter API client
      const socialConfig = this.configService.getSocialMediaConfig();
      if (socialConfig.twitter.apiKey && socialConfig.twitter.apiSecretKey) {
        this.twitterClient = new TwitterApi({
          appKey: socialConfig.twitter.apiKey,
          appSecret: socialConfig.twitter.apiSecretKey,
          accessToken: socialConfig.twitter.accessToken,
          accessSecret: socialConfig.twitter.accessTokenSecret,
        });
        console.log('✅ Twitter API client initialized');
      } else {
        console.warn('⚠️ Twitter API not configured');
      }

    } catch (error) {
      console.error('❌ Failed to initialize social media clients:', error);
    }
  }

  /**
   * Analyze an image using Microsoft Vision API
   */
  public async analyzeImage(imagePath: string): Promise<VisionAnalysisResult> {
    if (!this.visionClient) {
      throw new Error('Microsoft Vision API not configured');
    }

    try {
      console.log(`🔍 Analyzing image: ${imagePath}`);
      
      // Read image file
      const imageBuffer = await fs.readFile(imagePath);
      
      // Analyze image with Computer Vision
      const analysisResult = await this.visionClient.analyzeImageInStream(imageBuffer, {
        visualFeatures: [
          'Description',
          'Tags',
          'Objects',
          'Categories',
          'Color',
          'ImageType',
          'Faces'
        ],
        details: ['Landmarks']
      });

      // Extract text from image (OCR)
      const ocrResult = await this.visionClient.readInStream(imageBuffer);
      const operationId = ocrResult.operationLocation?.split('/').pop();
      
      let textResults: string[] = [];
      if (operationId) {
        // Poll for OCR results
        let ocrResults;
        do {
          await new Promise(resolve => setTimeout(resolve, 1000));
          ocrResults = await this.visionClient.getReadResult(operationId);
        } while (ocrResults.status === 'notStarted' || ocrResults.status === 'running');

        if (ocrResults.status === 'succeeded') {
          textResults = ocrResults.analyzeResult?.readResults
            ?.flatMap(page => page.lines?.map(line => line.text) || []) || [];
        }
      }

      const result: VisionAnalysisResult = {
        description: analysisResult.description?.captions?.[0]?.text || '',
        tags: analysisResult.tags?.map(tag => tag.name || '') || [],
        objects: analysisResult.objects?.map(obj => ({
          name: obj.objectProperty || '',
          confidence: obj.confidence || 0,
          boundingBox: {
            x: obj.rectangle?.x || 0,
            y: obj.rectangle?.y || 0,
            width: obj.rectangle?.w || 0,
            height: obj.rectangle?.h || 0
          }
        })) || [],
        text: textResults,
        faces: analysisResult.faces?.map(face => ({
          age: face.age || 0,
          gender: face.gender || 'unknown',
          emotion: 'neutral', // Emotion detection would require Face API
          confidence: 1.0
        })) || [],
        categories: analysisResult.categories?.map(cat => ({
          name: cat.name || '',
          confidence: cat.score || 0
        })) || [],
        color: {
          dominantColors: analysisResult.color?.dominantColors || [],
          accentColor: analysisResult.color?.accentColor || '',
          isBWImg: analysisResult.color?.isBWImg || false
        },
        imageType: {
          clipArtType: analysisResult.imageType?.clipArtType || 0,
          lineDrawingType: analysisResult.imageType?.lineDrawingType || 0
        }
      };

      console.log(`✅ Image analysis complete. Found: ${result.tags.length} tags, ${result.objects.length} objects, ${result.text.length} text elements`);
      return result;

    } catch (error) {
      console.error('❌ Image analysis failed:', error);
      throw new Error(`Image analysis failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  /**
   * Generate content based on analysis and parameters
   */
  public generateContent(options: ContentGenerationOptions): string {
    const { topic, tone, length, includeHashtags, platform, imageAnalysis } = options;

    let content = '';
    let characterLimit = 280; // Default to Twitter

    // Set character limits based on platform
    switch (platform) {
      case 'twitter':
        characterLimit = 280;
        break;
      case 'instagram':
        characterLimit = 2200;
        break;
      case 'linkedin':
        characterLimit = 1300;
        break;
    }

    // Generate base content based on topic and image analysis
    if (imageAnalysis) {
      const description = imageAnalysis.description;
      const mainTags = imageAnalysis.tags.slice(0, 3);
      const hasText = imageAnalysis.text.length > 0;

      switch (tone) {
        case 'professional':
          content = `Sharing insights on ${topic}. ${description ? `This image shows ${description}.` : ''} ${hasText ? 'Key information captured from the visual content.' : ''}`;
          break;
        case 'casual':
          content = `Check this out! ${description || `Something interesting about ${topic}`}. ${hasText ? 'Love the details in this!' : ''}`;
          break;
        case 'humorous':
          content = `When ${topic} meets reality 😄 ${description ? `This perfectly captures ${description}.` : ''} ${hasText ? 'The text says it all!' : ''}`;
          break;
        case 'informative':
          content = `Analysis: ${topic}. ${description || 'Visual content analyzed'} ${mainTags.length > 0 ? `Key elements: ${mainTags.join(', ')}.` : ''}`;
          break;
      }
    } else {
      // Generate content without image analysis
      switch (tone) {
        case 'professional':
          content = `Exploring ${topic} and its implications for modern workflows. Continuous innovation drives progress.`;
          break;
        case 'casual':
          content = `Just had some thoughts about ${topic}. Pretty interesting stuff if you ask me!`;
          break;
        case 'humorous':
          content = `${topic}: because sometimes you need a good laugh 😄 Life's too short to be serious all the time!`;
          break;
        case 'informative':
          content = `Understanding ${topic}: Key insights and considerations for effective implementation.`;
          break;
      }
    }

    // Adjust length
    if (length === 'short' && content.length > characterLimit / 2) {
      content = content.substring(0, characterLimit / 2 - 3) + '...';
    } else if (length === 'long' && platform !== 'twitter') {
      content += ` This represents an important development in the field and warrants careful consideration of the broader implications.`;
    }

    // Add hashtags if requested
    if (includeHashtags) {
      const hashtags: string[] = [];
      
      if (imageAnalysis) {
        // Generate hashtags from image tags
        hashtags.push(...imageAnalysis.tags.slice(0, 3).map(tag => 
          `#${tag.replace(/\s+/g, '').toLowerCase()}`
        ));
      }
      
      // Add topic-based hashtags
      hashtags.push(`#${topic.replace(/\s+/g, '').toLowerCase()}`);
      
      // Platform-specific hashtags
      switch (platform) {
        case 'twitter':
          hashtags.push('#automation', '#AI');
          break;
        case 'instagram':
          hashtags.push('#tech', '#innovation', '#digital');
          break;
        case 'linkedin':
          hashtags.push('#technology', '#business', '#innovation');
          break;
      }

      const hashtagText = hashtags.slice(0, 5).join(' ');
      
      // Ensure content + hashtags fit within character limit
      if (content.length + hashtagText.length + 2 <= characterLimit) {
        content += '\n\n' + hashtagText;
      }
    }

    // Final length check
    if (content.length > characterLimit) {
      content = content.substring(0, characterLimit - 3) + '...';
    }

    return content;
  }

  /**
   * Post content to Twitter
   */
  public async postToTwitter(content: string, mediaUrls?: string[]): Promise<PostResult> {
    if (!this.twitterClient) {
      throw new Error('Twitter API not configured');
    }

    try {
      console.log(`🐦 Posting to Twitter: ${content.substring(0, 50)}...`);

      let mediaIds: string[] = [];

      // Upload media if provided
      if (mediaUrls && mediaUrls.length > 0) {
        for (const mediaUrl of mediaUrls.slice(0, 4)) { // Twitter allows max 4 images
          try {
            let mediaBuffer: Buffer;
            
            if (mediaUrl.startsWith('http')) {
              // Download from URL
              const response = await axios.get(mediaUrl, { responseType: 'arraybuffer' });
              mediaBuffer = Buffer.from(response.data);
            } else {
              // Local file path
              mediaBuffer = await fs.readFile(mediaUrl);
            }

            const mediaUpload = await this.twitterClient.v1.uploadMedia(mediaBuffer, { 
              mimeType: 'image/png' 
            });
            mediaIds.push(mediaUpload);
          } catch (error) {
            console.warn(`⚠️ Failed to upload media: ${mediaUrl}`, error);
          }
        }
      }

      // Post the tweet
      const tweet = await this.twitterClient.v2.tweet({
        text: content,
        media: mediaIds.length > 0 ? { media_ids: mediaIds } : undefined
      });

      console.log(`✅ Posted to Twitter: ${tweet.data.id}`);

      return {
        success: true,
        postId: tweet.data.id,
        url: `https://twitter.com/user/status/${tweet.data.id}`
      };

    } catch (error) {
      console.error('❌ Twitter post failed:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }

  /**
   * Post content to Instagram (requires additional setup)
   */
  public async postToInstagram(content: string, mediaUrls?: string[]): Promise<PostResult> {
    // Instagram Basic Display API doesn't support posting
    // This would require Instagram Business API and additional setup
    console.warn('Instagram posting not implemented - requires Business API setup');
    
    return {
      success: false,
      error: 'Instagram posting requires Instagram Business API setup'
    };
  }

  /**
   * Post content to LinkedIn (requires additional setup)
   */
  public async postToLinkedIn(content: string, mediaUrls?: string[]): Promise<PostResult> {
    // LinkedIn API implementation would go here
    console.warn('LinkedIn posting not implemented');
    
    return {
      success: false,
      error: 'LinkedIn posting not yet implemented'
    };
  }

  /**
   * Post to multiple platforms
   */
  public async postToMultiplePlatforms(
    posts: SocialMediaPost[]
  ): Promise<Array<PostResult & { platform: string }>> {
    const results: Array<PostResult & { platform: string }> = [];

    for (const post of posts) {
      let result: PostResult;

      switch (post.platform) {
        case 'twitter':
          result = await this.postToTwitter(post.content, post.mediaUrls);
          break;
        case 'instagram':
          result = await this.postToInstagram(post.content, post.mediaUrls);
          break;
        case 'linkedin':
          result = await this.postToLinkedIn(post.content, post.mediaUrls);
          break;
        default:
          result = {
            success: false,
            error: `Unsupported platform: ${post.platform}`
          };
      }

      results.push({ ...result, platform: post.platform });
    }

    return results;
  }

  /**
   * Get engagement data for a post
   */
  public async getEngagementData(platform: string, postId: string): Promise<any> {
    switch (platform) {
      case 'twitter':
        if (!this.twitterClient) throw new Error('Twitter API not configured');
        
        try {
          const tweet = await this.twitterClient.v2.singleTweet(postId, {
            'tweet.fields': ['public_metrics', 'created_at']
          });

          return {
            likes: tweet.data.public_metrics?.like_count || 0,
            retweets: tweet.data.public_metrics?.retweet_count || 0,
            replies: tweet.data.public_metrics?.reply_count || 0,
            views: tweet.data.public_metrics?.impression_count || 0
          };
        } catch (error) {
          console.error('Failed to get Twitter engagement data:', error);
          return null;
        }

      default:
        throw new Error(`Engagement data not supported for platform: ${platform}`);
    }
  }

  /**
   * Schedule a post (basic implementation - would need job queue in production)
   */
  public async schedulePost(post: SocialMediaPost): Promise<string> {
    if (!post.scheduledFor) {
      throw new Error('Scheduled time required');
    }

    const now = new Date();
    const scheduledTime = new Date(post.scheduledFor);
    const delay = scheduledTime.getTime() - now.getTime();

    if (delay <= 0) {
      throw new Error('Scheduled time must be in the future');
    }

    // Simple setTimeout implementation - in production, use a proper job queue
    const scheduleId = `schedule_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    setTimeout(async () => {
      console.log(`⏰ Executing scheduled post: ${scheduleId}`);
      
      try {
        switch (post.platform) {
          case 'twitter':
            await this.postToTwitter(post.content, post.mediaUrls);
            break;
          case 'instagram':
            await this.postToInstagram(post.content, post.mediaUrls);
            break;
          case 'linkedin':
            await this.postToLinkedIn(post.content, post.mediaUrls);
            break;
        }
        console.log(`✅ Scheduled post executed: ${scheduleId}`);
      } catch (error) {
        console.error(`❌ Scheduled post failed: ${scheduleId}`, error);
      }
    }, delay);

    console.log(`📅 Post scheduled for ${scheduledTime.toISOString()} (ID: ${scheduleId})`);
    return scheduleId;
  }

  /**
   * Analyze image and generate social media content
   */
  public async analyzeAndGenerateContent(
    imagePath: string,
    options: Omit<ContentGenerationOptions, 'imageAnalysis'>
  ): Promise<{
    analysis: VisionAnalysisResult;
    content: string;
  }> {
    const analysis = await this.analyzeImage(imagePath);
    const content = this.generateContent({ ...options, imageAnalysis: analysis });

    return { analysis, content };
  }

  // Service status methods
  public getStatus(): {
    visionApiAvailable: boolean;
    twitterApiAvailable: boolean;
    instagramApiAvailable: boolean;
    linkedinApiAvailable: boolean;
  } {
    return {
      visionApiAvailable: !!this.visionClient,
      twitterApiAvailable: !!this.twitterClient,
      instagramApiAvailable: false, // Not implemented
      linkedinApiAvailable: false   // Not implemented
    };
  }

  public isVisionApiAvailable(): boolean {
    return !!this.visionClient;
  }

  public isTwitterApiAvailable(): boolean {
    return !!this.twitterClient;
  }
}