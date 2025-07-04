import express from 'express';
import { AgentService } from '../services/AgentService.js';

const router = express.Router();

// Note: In a real application, you would inject these services through dependency injection
// For this example, we'll access them through the global app context or similar mechanism

// GET /api/agent/status
router.get('/status', async (req, res) => {
  try {
    // This would normally be injected - for demo purposes, we'll access it through app context
    const agentService = req.app.get('agentService') as AgentService;
    const status = agentService.getStatus();
    res.json(status);
  } catch (error) {
    console.error('Failed to get agent status:', error);
    res.status(500).json({ 
      error: 'Failed to get agent status',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/start
router.post('/start', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    await agentService.start();
    res.json({ success: true, message: 'Agent started successfully' });
  } catch (error) {
    console.error('Failed to start agent:', error);
    res.status(500).json({ 
      error: 'Failed to start agent',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/stop
router.post('/stop', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    await agentService.stop();
    res.json({ success: true, message: 'Agent stopped successfully' });
  } catch (error) {
    console.error('Failed to stop agent:', error);
    res.status(500).json({ 
      error: 'Failed to stop agent',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/agent/tasks
router.get('/tasks', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    
    const options = {
      status: req.query.status as string,
      type: req.query.type as string,
      limit: req.query.limit ? parseInt(req.query.limit as string) : undefined,
      offset: req.query.offset ? parseInt(req.query.offset as string) : undefined,
      orderBy: req.query.orderBy as string,
      orderDirection: req.query.orderDirection as 'ASC' | 'DESC'
    };

    const tasks = await agentService.getTasks(options);
    res.json(tasks);
  } catch (error) {
    console.error('Failed to get tasks:', error);
    res.status(500).json({ 
      error: 'Failed to get tasks',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/tasks
router.post('/tasks', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    
    const { name, type, priority = 1, parameters = {}, maxRetries = 3 } = req.body;

    if (!name || !type) {
      return res.status(400).json({ 
        error: 'Missing required fields',
        message: 'name and type are required'
      });
    }

    const taskId = await agentService.addTask({
      name,
      type,
      priority,
      parameters,
      maxRetries
    });

    res.status(201).json({ 
      success: true, 
      taskId,
      message: 'Task added successfully' 
    });
  } catch (error) {
    console.error('Failed to add task:', error);
    res.status(500).json({ 
      error: 'Failed to add task',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// DELETE /api/agent/tasks/:taskId
router.delete('/tasks/:taskId', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    const { taskId } = req.params;
    
    const cancelled = await agentService.cancelTask(taskId);
    
    if (cancelled) {
      res.json({ success: true, message: 'Task cancelled successfully' });
    } else {
      res.status(404).json({ 
        error: 'Task not found',
        message: 'Task not found or cannot be cancelled'
      });
    }
  } catch (error) {
    console.error('Failed to cancel task:', error);
    res.status(500).json({ 
      error: 'Failed to cancel task',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/agent/automation/rules
router.get('/automation/rules', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    const rules = agentService.getAutomationRules();
    res.json(rules);
  } catch (error) {
    console.error('Failed to get automation rules:', error);
    res.status(500).json({ 
      error: 'Failed to get automation rules',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/automation/rules
router.post('/automation/rules', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    
    const { name, description, trigger, conditions = [], actions = [], isActive = true } = req.body;

    if (!name || !trigger || !actions.length) {
      return res.status(400).json({ 
        error: 'Missing required fields',
        message: 'name, trigger, and actions are required'
      });
    }

    const ruleId = agentService.addAutomationRule({
      name,
      description,
      trigger,
      conditions,
      actions,
      isActive
    });

    res.status(201).json({ 
      success: true, 
      ruleId,
      message: 'Automation rule added successfully' 
    });
  } catch (error) {
    console.error('Failed to add automation rule:', error);
    res.status(500).json({ 
      error: 'Failed to add automation rule',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/agent/automation/rules/:ruleId
router.get('/automation/rules/:ruleId', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    const { ruleId } = req.params;
    
    const rule = agentService.getAutomationRule(ruleId);
    
    if (rule) {
      res.json(rule);
    } else {
      res.status(404).json({ 
        error: 'Rule not found',
        message: 'Automation rule not found'
      });
    }
  } catch (error) {
    console.error('Failed to get automation rule:', error);
    res.status(500).json({ 
      error: 'Failed to get automation rule',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// DELETE /api/agent/automation/rules/:ruleId
router.delete('/automation/rules/:ruleId', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    const { ruleId } = req.params;
    
    const deleted = await agentService.deleteAutomationRule(ruleId);
    
    if (deleted) {
      res.json({ success: true, message: 'Automation rule deleted successfully' });
    } else {
      res.status(404).json({ 
        error: 'Rule not found',
        message: 'Automation rule not found'
      });
    }
  } catch (error) {
    console.error('Failed to delete automation rule:', error);
    res.status(500).json({ 
      error: 'Failed to delete automation rule',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/config
router.post('/config', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    const { maxConcurrentTasks } = req.body;
    
    if (maxConcurrentTasks !== undefined) {
      agentService.setMaxConcurrentTasks(maxConcurrentTasks);
    }
    
    res.json({ 
      success: true, 
      message: 'Configuration updated successfully',
      config: {
        maxConcurrentTasks
      }
    });
  } catch (error) {
    console.error('Failed to update configuration:', error);
    res.status(500).json({ 
      error: 'Failed to update configuration',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/tasks/screen-capture
router.post('/tasks/screen-capture', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    
    const { 
      name = 'Manual Screen Capture',
      priority = 1,
      options = {} 
    } = req.body;

    const taskId = await agentService.addTask({
      name,
      type: 'screen_capture',
      priority,
      parameters: options
    });

    res.status(201).json({ 
      success: true, 
      taskId,
      message: 'Screen capture task added successfully' 
    });
  } catch (error) {
    console.error('Failed to add screen capture task:', error);
    res.status(500).json({ 
      error: 'Failed to add screen capture task',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/tasks/social-media
router.post('/tasks/social-media', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    
    const { 
      name = 'Social Media Task',
      priority = 1,
      action,
      platform,
      content,
      mediaUrls,
      contentOptions,
      imagePath
    } = req.body;

    if (!action) {
      return res.status(400).json({ 
        error: 'Missing required field',
        message: 'action is required'
      });
    }

    const taskId = await agentService.addTask({
      name,
      type: 'social_media',
      priority,
      parameters: {
        action,
        platform,
        content,
        mediaUrls,
        contentOptions,
        imagePath
      }
    });

    res.status(201).json({ 
      success: true, 
      taskId,
      message: 'Social media task added successfully' 
    });
  } catch (error) {
    console.error('Failed to add social media task:', error);
    res.status(500).json({ 
      error: 'Failed to add social media task',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/agent/tasks/automation
router.post('/tasks/automation', async (req, res) => {
  try {
    const agentService = req.app.get('agentService') as AgentService;
    
    const { 
      name = 'Automation Task',
      priority = 1,
      ruleId,
      action = 'execute_rule'
    } = req.body;

    if (!ruleId) {
      return res.status(400).json({ 
        error: 'Missing required field',
        message: 'ruleId is required'
      });
    }

    const taskId = await agentService.addTask({
      name,
      type: 'automation',
      priority,
      parameters: {
        ruleId,
        action
      }
    });

    res.status(201).json({ 
      success: true, 
      taskId,
      message: 'Automation task added successfully' 
    });
  } catch (error) {
    console.error('Failed to add automation task:', error);
    res.status(500).json({ 
      error: 'Failed to add automation task',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

export default router;