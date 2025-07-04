import express from 'express';
import { SocialMediaService } from '../services/SocialMediaService.js';
import { DatabaseService } from '../services/DatabaseService.js';
import multer from 'multer';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const router = express.Router();

// Configure multer for file uploads
const storage = multer.diskStorage({
  destination: (req, file, cb) => {
    cb(null, path.join(__dirname, '../../../data/uploads/'));
  },
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, file.fieldname + '-' + uniqueSuffix + path.extname(file.originalname));
  }
});

const upload = multer({ 
  storage: storage,
  limits: {
    fileSize: 10 * 1024 * 1024, // 10MB limit
  },
  fileFilter: (req, file, cb) => {
    const allowedTypes = /jpeg|jpg|png|gif|webp/;
    const extname = allowedTypes.test(path.extname(file.originalname).toLowerCase());
    const mimetype = allowedTypes.test(file.mimetype);

    if (mimetype && extname) {
      return cb(null, true);
    } else {
      cb(new Error('Only image files are allowed'));
    }
  }
});

// GET /api/social/status
router.get('/status', async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const status = socialMediaService.getStatus();
    
    res.json({
      success: true,
      status
    });
  } catch (error) {
    console.error('Failed to get social media status:', error);
    res.status(500).json({ 
      error: 'Failed to get social media status',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/social/analyze/image
router.post('/analyze/image', upload.single('image'), async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    
    if (!req.file) {
      return res.status(400).json({ 
        error: 'No image provided',
        message: 'Please upload an image file'
      });
    }

    const analysis = await socialMediaService.analyzeImage(req.file.path);
    
    res.json({
      success: true,
      analysis,
      imageInfo: {
        filename: req.file.filename,
        originalName: req.file.originalname,
        size: req.file.size,
        path: req.file.path
      }
    });
  } catch (error) {
    console.error('Failed to analyze image:', error);
    res.status(500).json({ 
      error: 'Failed to analyze image',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/social/analyze/url
router.post('/analyze/url', async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const { imageUrl } = req.body;

    if (!imageUrl) {
      return res.status(400).json({ 
        error: 'Missing image URL',
        message: 'imageUrl is required'
      });
    }

    // Download image first (simplified - in production, add proper validation and error handling)
    const analysis = await socialMediaService.analyzeImage(imageUrl);
    
    res.json({
      success: true,
      analysis
    });
  } catch (error) {
    console.error('Failed to analyze image from URL:', error);
    res.status(500).json({ 
      error: 'Failed to analyze image from URL',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/social/content/generate
router.post('/content/generate', async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    
    const { 
      topic, 
      tone = 'professional', 
      length = 'medium', 
      includeHashtags = true, 
      platform = 'twitter',
      imageAnalysis 
    } = req.body;

    if (!topic) {
      return res.status(400).json({ 
        error: 'Missing topic',
        message: 'topic is required for content generation'
      });
    }

    const content = socialMediaService.generateContent({
      topic,
      tone,
      length,
      includeHashtags,
      platform,
      imageAnalysis
    });
    
    res.json({
      success: true,
      content,
      parameters: {
        topic,
        tone,
        length,
        includeHashtags,
        platform
      }
    });
  } catch (error) {
    console.error('Failed to generate content:', error);
    res.status(500).json({ 
      error: 'Failed to generate content',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/social/post/twitter
router.post('/post/twitter', upload.array('media', 4), async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    const { content } = req.body;

    if (!content) {
      return res.status(400).json({ 
        error: 'Missing content',
        message: 'content is required for posting'
      });
    }

    const mediaFiles = req.files as Express.Multer.File[];
    const mediaUrls = mediaFiles?.map(file => file.path) || [];

    const result = await socialMediaService.postToTwitter(content, mediaUrls);
    
    // Save to database
    await databaseService.saveSocialMediaPost({
      platform: 'twitter',
      content,
      mediaUrls: mediaUrls.length > 0 ? JSON.stringify(mediaUrls) : undefined,
      status: result.success ? 'posted' : 'failed',
      postId: result.postId,
      postedAt: result.success ? new Date() : undefined
    });
    
    res.json({
      success: result.success,
      result,
      mediaCount: mediaUrls.length
    });
  } catch (error) {
    console.error('Failed to post to Twitter:', error);
    res.status(500).json({ 
      error: 'Failed to post to Twitter',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/social/post/multiple
router.post('/post/multiple', upload.array('media', 10), async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    const { posts } = req.body;

    if (!posts || !Array.isArray(posts)) {
      return res.status(400).json({ 
        error: 'Invalid posts data',
        message: 'posts array is required'
      });
    }

    const mediaFiles = req.files as Express.Multer.File[];
    const mediaUrls = mediaFiles?.map(file => file.path) || [];

    // Add media URLs to posts if provided
    const postsWithMedia = posts.map((post, index) => ({
      ...post,
      mediaUrls: mediaUrls.slice(index * 4, (index + 1) * 4) // Max 4 media per post
    }));

    const results = await socialMediaService.postToMultiplePlatforms(postsWithMedia);
    
    // Save all posts to database
    for (let i = 0; i < results.length; i++) {
      const result = results[i];
      const post = postsWithMedia[i];
      
      await databaseService.saveSocialMediaPost({
        platform: result.platform as any,
        content: post.content,
        mediaUrls: post.mediaUrls && post.mediaUrls.length > 0 ? JSON.stringify(post.mediaUrls) : undefined,
        status: result.success ? 'posted' : 'failed',
        postId: result.postId,
        postedAt: result.success ? new Date() : undefined
      });
    }
    
    res.json({
      success: true,
      results,
      totalPosts: results.length,
      successfulPosts: results.filter(r => r.success).length
    });
  } catch (error) {
    console.error('Failed to post to multiple platforms:', error);
    res.status(500).json({ 
      error: 'Failed to post to multiple platforms',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/social/schedule
router.post('/schedule', upload.array('media', 4), async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    const { platform, content, scheduledFor } = req.body;

    if (!platform || !content || !scheduledFor) {
      return res.status(400).json({ 
        error: 'Missing required fields',
        message: 'platform, content, and scheduledFor are required'
      });
    }

    const mediaFiles = req.files as Express.Multer.File[];
    const mediaUrls = mediaFiles?.map(file => file.path) || [];

    const post = {
      platform,
      content,
      mediaUrls,
      scheduledFor: new Date(scheduledFor)
    };

    const scheduleId = await socialMediaService.schedulePost(post);
    
    // Save to database as scheduled
    await databaseService.saveSocialMediaPost({
      platform,
      content,
      mediaUrls: mediaUrls.length > 0 ? JSON.stringify(mediaUrls) : undefined,
      status: 'scheduled',
      scheduledFor: new Date(scheduledFor)
    });
    
    res.json({
      success: true,
      scheduleId,
      scheduledFor: post.scheduledFor,
      message: 'Post scheduled successfully'
    });
  } catch (error) {
    console.error('Failed to schedule post:', error);
    res.status(500).json({ 
      error: 'Failed to schedule post',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/social/analyze-and-post
router.post('/analyze-and-post', upload.single('image'), async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    const { 
      topic, 
      tone = 'professional', 
      length = 'medium', 
      includeHashtags = true, 
      platform = 'twitter' 
    } = req.body;

    if (!req.file) {
      return res.status(400).json({ 
        error: 'No image provided',
        message: 'Please upload an image file for analysis'
      });
    }

    if (!topic) {
      return res.status(400).json({ 
        error: 'Missing topic',
        message: 'topic is required for content generation'
      });
    }

    const { analysis, content } = await socialMediaService.analyzeAndGenerateContent(
      req.file.path,
      { topic, tone, length, includeHashtags, platform }
    );

    // Post to the specified platform
    let postResult;
    switch (platform) {
      case 'twitter':
        postResult = await socialMediaService.postToTwitter(content, [req.file.path]);
        break;
      default:
        throw new Error(`Posting to ${platform} not implemented yet`);
    }

    // Save to database
    await databaseService.saveSocialMediaPost({
      platform,
      content,
      mediaUrls: JSON.stringify([req.file.path]),
      status: postResult.success ? 'posted' : 'failed',
      postId: postResult.postId,
      postedAt: postResult.success ? new Date() : undefined
    });
    
    res.json({
      success: true,
      analysis,
      content,
      postResult,
      imageInfo: {
        filename: req.file.filename,
        originalName: req.file.originalname,
        size: req.file.size
      }
    });
  } catch (error) {
    console.error('Failed to analyze and post:', error);
    res.status(500).json({ 
      error: 'Failed to analyze and post',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/social/posts
router.get('/posts', async (req, res) => {
  try {
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    // This would require implementing a getPosts method in DatabaseService
    // For now, return placeholder data
    const posts = []; // await databaseService.getSocialMediaPosts(options);
    
    res.json({
      success: true,
      posts,
      message: 'Social media posts endpoint (placeholder - needs database implementation)'
    });
  } catch (error) {
    console.error('Failed to get posts:', error);
    res.status(500).json({ 
      error: 'Failed to get posts',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/social/engagement/:platform/:postId
router.get('/engagement/:platform/:postId', async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const { platform, postId } = req.params;

    const engagementData = await socialMediaService.getEngagementData(platform, postId);
    
    if (engagementData) {
      res.json({
        success: true,
        platform,
        postId,
        engagement: engagementData
      });
    } else {
      res.status(404).json({
        error: 'Engagement data not found',
        message: 'Could not retrieve engagement data for the specified post'
      });
    }
  } catch (error) {
    console.error('Failed to get engagement data:', error);
    res.status(500).json({ 
      error: 'Failed to get engagement data',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/social/platforms
router.get('/platforms', async (req, res) => {
  try {
    const socialMediaService = req.app.get('socialMediaService') as SocialMediaService;
    const status = socialMediaService.getStatus();
    
    const platforms = [
      {
        name: 'twitter',
        displayName: 'Twitter',
        available: status.twitterApiAvailable,
        features: ['post', 'media', 'engagement'],
        limits: {
          textLength: 280,
          mediaCount: 4,
          mediaSize: '5MB'
        }
      },
      {
        name: 'instagram',
        displayName: 'Instagram',
        available: status.instagramApiAvailable,
        features: ['post', 'media'],
        limits: {
          textLength: 2200,
          mediaCount: 10,
          mediaSize: '100MB'
        },
        note: 'Requires Business API setup'
      },
      {
        name: 'linkedin',
        displayName: 'LinkedIn',
        available: status.linkedinApiAvailable,
        features: ['post', 'media'],
        limits: {
          textLength: 1300,
          mediaCount: 9,
          mediaSize: '100MB'
        },
        note: 'Not yet implemented'
      }
    ];
    
    res.json({
      success: true,
      platforms,
      visionAnalysis: status.visionApiAvailable
    });
  } catch (error) {
    console.error('Failed to get platform info:', error);
    res.status(500).json({ 
      error: 'Failed to get platform info',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// Error handling middleware for multer
router.use((error: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
  if (error instanceof multer.MulterError) {
    if (error.code === 'LIMIT_FILE_SIZE') {
      return res.status(400).json({
        error: 'File too large',
        message: 'File size must be less than 10MB'
      });
    }
    return res.status(400).json({
      error: 'File upload error',
      message: error.message
    });
  }
  
  if (error.message === 'Only image files are allowed') {
    return res.status(400).json({
      error: 'Invalid file type',
      message: 'Only image files (JPEG, PNG, GIF, WebP) are allowed'
    });
  }
  
  next(error);
});

export default router;