import { useState, useEffect, useRef } from 'react';
import { Socket } from 'socket.io-client';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { 
  Activity, 
  CheckCircle, 
  XCircle, 
  Clock, 
  AlertTriangle,
  Info,
  Zap,
  RefreshCw,
  Filter
} from 'lucide-react';

interface ActivityFeedProps {
  socket: Socket | null;
}

interface ActivityItem {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  category: 'system' | 'agent' | 'user' | 'api';
  message: string;
  details?: string;
  timestamp: Date;
  source: string;
  taskId?: string;
}

interface Task {
  id: string;
  name: string;
  type: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  startTime?: Date;
  endTime?: Date;
  duration?: number;
}

export function ActivityFeed({ socket }: ActivityFeedProps) {
  const [activities, setActivities] = useState<ActivityItem[]>([]);
  const [filter, setFilter] = useState<string>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const scrollAreaRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!socket) return;

    // Mock initial activity data
    const mockActivities: ActivityItem[] = [
      {
        id: '1',
        type: 'success',
        category: 'agent',
        message: 'Agent service started successfully',
        timestamp: new Date(Date.now() - 300000),
        source: 'AgentService'
      },
      {
        id: '2',
        type: 'info',
        category: 'system',
        message: 'Screen capture service initialized',
        timestamp: new Date(Date.now() - 250000),
        source: 'ScreenCaptureService'
      },
      {
        id: '3',
        type: 'info',
        category: 'system',
        message: 'Database connection established',
        timestamp: new Date(Date.now() - 200000),
        source: 'DatabaseService'
      }
    ];

    setActivities(mockActivities);

    // Socket event handlers
    const handleTaskStarted = (task: Task) => {
      const activity: ActivityItem = {
        id: `task-start-${task.id}`,
        type: 'info',
        category: 'agent',
        message: `Task started: ${task.name}`,
        details: `Type: ${task.type}`,
        timestamp: new Date(),
        source: 'AgentService',
        taskId: task.id
      };
      addActivity(activity);
    };

    const handleTaskCompleted = (task: Task) => {
      const activity: ActivityItem = {
        id: `task-complete-${task.id}`,
        type: 'success',
        category: 'agent',
        message: `Task completed: ${task.name}`,
        details: task.duration ? `Duration: ${task.duration}ms` : undefined,
        timestamp: new Date(),
        source: 'AgentService',
        taskId: task.id
      };
      addActivity(activity);
    };

    const handleTaskFailed = (task: Task) => {
      const activity: ActivityItem = {
        id: `task-failed-${task.id}`,
        type: 'error',
        category: 'agent',
        message: `Task failed: ${task.name}`,
        details: task.duration ? `Duration: ${task.duration}ms` : undefined,
        timestamp: new Date(),
        source: 'AgentService',
        taskId: task.id
      };
      addActivity(activity);
    };

    const handleTaskCancelled = (task: Task) => {
      const activity: ActivityItem = {
        id: `task-cancelled-${task.id}`,
        type: 'warning',
        category: 'agent',
        message: `Task cancelled: ${task.name}`,
        timestamp: new Date(),
        source: 'AgentService',
        taskId: task.id
      };
      addActivity(activity);
    };

    const handleAgentStarted = () => {
      const activity: ActivityItem = {
        id: `agent-started-${Date.now()}`,
        type: 'success',
        category: 'agent',
        message: 'Agent service started',
        timestamp: new Date(),
        source: 'AgentService'
      };
      addActivity(activity);
    };

    const handleAgentStopped = () => {
      const activity: ActivityItem = {
        id: `agent-stopped-${Date.now()}`,
        type: 'warning',
        category: 'agent',
        message: 'Agent service stopped',
        timestamp: new Date(),
        source: 'AgentService'
      };
      addActivity(activity);
    };

    const handleScreenCaptured = (result: any) => {
      const activity: ActivityItem = {
        id: `screen-captured-${Date.now()}`,
        type: 'info',
        category: 'system',
        message: 'Screenshot captured',
        details: `Size: ${result.metadata?.width}x${result.metadata?.height}`,
        timestamp: new Date(),
        source: 'ScreenCaptureService'
      };
      addActivity(activity);
    };

    const handleError = (error: { message: string }) => {
      const activity: ActivityItem = {
        id: `error-${Date.now()}`,
        type: 'error',
        category: 'system',
        message: `Error: ${error.message}`,
        timestamp: new Date(),
        source: 'System'
      };
      addActivity(activity);
    };

    // Register event listeners
    socket.on('task:started', handleTaskStarted);
    socket.on('task:completed', handleTaskCompleted);
    socket.on('task:failed', handleTaskFailed);
    socket.on('task:cancelled', handleTaskCancelled);
    socket.on('agent:started', handleAgentStarted);
    socket.on('agent:stopped', handleAgentStopped);
    socket.on('screen:captured', handleScreenCaptured);
    socket.on('error', handleError);

    return () => {
      socket.off('task:started', handleTaskStarted);
      socket.off('task:completed', handleTaskCompleted);
      socket.off('task:failed', handleTaskFailed);
      socket.off('task:cancelled', handleTaskCancelled);
      socket.off('agent:started', handleAgentStarted);
      socket.off('agent:stopped', handleAgentStopped);
      socket.off('screen:captured', handleScreenCaptured);
      socket.off('error', handleError);
    };
  }, [socket]);

  const addActivity = (activity: ActivityItem) => {
    setActivities(prev => [activity, ...prev].slice(0, 100)); // Keep last 100 activities
  };

  useEffect(() => {
    if (autoScroll && scrollAreaRef.current) {
      scrollAreaRef.current.scrollTop = 0;
    }
  }, [activities, autoScroll]);

  const getIcon = (type: string) => {
    switch (type) {
      case 'success':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'error':
        return <XCircle className="h-4 w-4 text-red-500" />;
      case 'warning':
        return <AlertTriangle className="h-4 w-4 text-yellow-500" />;
      default:
        return <Info className="h-4 w-4 text-blue-500" />;
    }
  };

  const getBadgeVariant = (type: string) => {
    switch (type) {
      case 'success':
        return 'default';
      case 'error':
        return 'destructive';
      case 'warning':
        return 'secondary';
      default:
        return 'outline';
    }
  };

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  };

  const formatRelativeTime = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) return `${hours}h ago`;
    if (minutes > 0) return `${minutes}m ago`;
    return `${seconds}s ago`;
  };

  const filteredActivities = activities.filter(activity => {
    if (filter === 'all') return true;
    return activity.type === filter || activity.category === filter;
  });

  const clearActivities = () => {
    setActivities([]);
  };

  return (
    <Card className="h-[600px] flex flex-col">
      <CardHeader className="flex-none">
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              Activity Feed
            </CardTitle>
            <CardDescription>
              Real-time system events and task updates
            </CardDescription>
          </div>
          
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setAutoScroll(!autoScroll)}
            >
              {autoScroll ? (
                <>
                  <Zap className="h-4 w-4 mr-2" />
                  Live
                </>
              ) : (
                <>
                  <RefreshCw className="h-4 w-4 mr-2" />
                  Paused
                </>
              )}
            </Button>
            
            <Button
              variant="outline"
              size="sm"
              onClick={clearActivities}
            >
              Clear
            </Button>
          </div>
        </div>

        {/* Filter Buttons */}
        <div className="flex flex-wrap gap-2">
          {['all', 'success', 'error', 'warning', 'info', 'agent', 'system'].map((filterType) => (
            <Button
              key={filterType}
              variant={filter === filterType ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter(filterType)}
              className="text-xs h-7"
            >
              {filterType === 'all' ? (
                <Filter className="h-3 w-3 mr-1" />
              ) : null}
              {filterType.charAt(0).toUpperCase() + filterType.slice(1)}
            </Button>
          ))}
        </div>
      </CardHeader>

      <CardContent className="flex-1 min-h-0 p-0">
        <ScrollArea className="h-full px-6 pb-6" ref={scrollAreaRef}>
          <div className="space-y-3">
            {filteredActivities.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Activity className="h-8 w-8 mx-auto mb-2 opacity-50" />
                <p>No activities to show</p>
                <p className="text-sm">Events will appear here as they happen</p>
              </div>
            ) : (
              filteredActivities.map((activity, index) => (
                <div
                  key={activity.id}
                  className={`flex gap-3 p-3 rounded-lg border transition-colors ${
                    index === 0 && autoScroll ? 'bg-primary/5 border-primary/20' : 'bg-muted/20'
                  }`}
                >
                  <div className="flex-none mt-0.5">
                    {getIcon(activity.type)}
                  </div>
                  
                  <div className="flex-1 min-w-0">
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex-1">
                        <p className="text-sm font-medium leading-tight">
                          {activity.message}
                        </p>
                        {activity.details && (
                          <p className="text-xs text-muted-foreground mt-1">
                            {activity.details}
                          </p>
                        )}
                      </div>
                      
                      <div className="flex-none flex flex-col items-end gap-1">
                        <Badge variant={getBadgeVariant(activity.type)} className="text-xs">
                          {activity.category}
                        </Badge>
                        <div className="text-xs text-muted-foreground">
                          {formatTime(activity.timestamp)}
                        </div>
                      </div>
                    </div>
                    
                    <div className="flex items-center justify-between mt-2">
                      <span className="text-xs text-muted-foreground font-mono">
                        {activity.source}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {formatRelativeTime(activity.timestamp)}
                      </span>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}