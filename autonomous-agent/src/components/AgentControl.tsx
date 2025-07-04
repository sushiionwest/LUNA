import { useState, useEffect } from 'react';
import { Socket } from 'socket.io-client';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Separator } from '@/components/ui/separator';
import { 
  Play, 
  Pause, 
  RotateCcw, 
  Settings, 
  Activity, 
  Clock,
  Zap,
  AlertTriangle,
  CheckCircle
} from 'lucide-react';

interface AgentControlProps {
  socket: Socket | null;
}

interface AgentStatus {
  isRunning: boolean;
  currentTasks: number;
  maxConcurrentTasks: number;
  totalTasksProcessed: number;
  successfulTasks: number;
  failedTasks: number;
  queuedTasks: number;
  uptime: number;
  lastActivity: Date | null;
}

export function LunaControl({ socket }: AgentControlProps) {
  const [agentStatus, setAgentStatus] = useState<AgentStatus | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    if (!socket) return;

    const handleAgentStatus = (status: AgentStatus) => {
      setAgentStatus(status);
      setIsLoading(false);
    };

    const handleAgentStarted = () => {
      setIsLoading(false);
      console.log('Agent started successfully');
    };

    const handleAgentStopped = () => {
      setIsLoading(false);
      console.log('Agent stopped successfully');
    };

    const handleError = (error: { message: string }) => {
      setIsLoading(false);
      console.error('Agent operation failed:', error.message);
    };

    socket.on('agent:status', handleAgentStatus);
    socket.on('agent:started', handleAgentStarted);
    socket.on('agent:stopped', handleAgentStopped);
    socket.on('error', handleError);

    // Request initial status
    socket.emit('status:get');

    return () => {
      socket.off('agent:status', handleAgentStatus);
      socket.off('agent:started', handleAgentStarted);
      socket.off('agent:stopped', handleAgentStopped);
      socket.off('error', handleError);
    };
  }, [socket]);

  const startAgent = () => {
    if (!socket) return;
    setIsLoading(true);
    socket.emit('agent:start');
  };

  const stopAgent = () => {
    if (!socket) return;
    setIsLoading(true);
    socket.emit('agent:stop');
  };

  const restartAgent = () => {
    if (!socket) return;
    setIsLoading(true);
    socket.emit('agent:stop');
    setTimeout(() => {
      socket.emit('agent:start');
    }, 1000);
  };

  const formatUptime = (uptimeMs: number) => {
    const seconds = Math.floor(uptimeMs / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d ${hours % 24}h`;
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    return `${seconds}s`;
  };

  const getSuccessRate = () => {
    if (!agentStatus || agentStatus.totalTasksProcessed === 0) return 0;
    return Math.round((agentStatus.successfulTasks / agentStatus.totalTasksProcessed) * 100);
  };

  const getTaskUtilization = () => {
    if (!agentStatus || agentStatus.maxConcurrentTasks === 0) return 0;
    return Math.round((agentStatus.currentTasks / agentStatus.maxConcurrentTasks) * 100);
  };

  return (
    <div className="space-y-6">
      {/* Main Control Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <span className="text-2xl">🌙</span>
            Luna Control
          </CardTitle>
          <CardDescription>
            Wake Luna up, let her rest, and see what she's working on
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Status Display */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className={`h-3 w-3 rounded-full ${
                agentStatus?.isRunning 
                  ? 'bg-green-500 animate-pulse' 
                  : 'bg-gray-400'
              }`} />
              <div>
                <div className="font-medium">
                  {agentStatus?.isRunning ? 'Luna is Watching' : 'Luna is Resting'}
                </div>
                <div className="text-sm text-muted-foreground">
                  {agentStatus?.lastActivity 
                    ? `Last activity: ${new Date(agentStatus.lastActivity).toLocaleTimeString()}`
                    : 'No recent activity'
                  }
                </div>
              </div>
            </div>
            
            <Badge variant={agentStatus?.isRunning ? 'default' : 'secondary'}>
              {agentStatus?.isRunning ? 'Watching' : 'Sleeping'}
            </Badge>
          </div>

          {/* Control Buttons */}
          <div className="flex gap-2">
            {agentStatus?.isRunning ? (
              <Button 
                variant="destructive" 
                onClick={stopAgent}
                disabled={isLoading}
                className="flex-1"
              >
                <Pause className="h-4 w-4 mr-2" />
                {isLoading ? 'Luna is resting...' : 'Let Luna Rest'}
              </Button>
            ) : (
              <Button 
                onClick={startAgent}
                disabled={isLoading}
                className="flex-1"
              >
                <Play className="h-4 w-4 mr-2" />
                {isLoading ? 'Luna is waking up...' : 'Wake Luna Up'}
              </Button>
            )}
            
            <Button 
              variant="outline" 
              onClick={restartAgent}
              disabled={isLoading}
            >
              <RotateCcw className="h-4 w-4 mr-2" />
              Refresh Luna
            </Button>
            
            <Button variant="outline" size="icon">
              <Settings className="h-4 w-4" />
            </Button>
          </div>

          {agentStatus && (
            <>
              <Separator />
              
              {/* Performance Metrics */}
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span>Task Utilization</span>
                    <span>{agentStatus.currentTasks}/{agentStatus.maxConcurrentTasks}</span>
                  </div>
                  <Progress value={getTaskUtilization()} className="h-2" />
                </div>
                
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span>Success Rate</span>
                    <span>{getSuccessRate()}%</span>
                  </div>
                  <Progress value={getSuccessRate()} className="h-2" />
                </div>
              </div>

              {/* Statistics */}
              <div className="grid gap-4 md:grid-cols-3">
                <div className="text-center p-3 bg-muted/50 rounded-lg">
                  <div className="text-2xl font-bold text-green-600">
                    {agentStatus.successfulTasks}
                  </div>
                  <div className="text-sm text-muted-foreground">Successful</div>
                </div>
                
                <div className="text-center p-3 bg-muted/50 rounded-lg">
                  <div className="text-2xl font-bold text-red-600">
                    {agentStatus.failedTasks}
                  </div>
                  <div className="text-sm text-muted-foreground">Failed</div>
                </div>
                
                <div className="text-center p-3 bg-muted/50 rounded-lg">
                  <div className="text-2xl font-bold text-blue-600">
                    {agentStatus.queuedTasks}
                  </div>
                  <div className="text-sm text-muted-foreground">Queued</div>
                </div>
              </div>

              {/* Uptime */}
              <div className="flex items-center justify-between p-3 bg-muted/30 rounded-lg">
                <div className="flex items-center gap-2">
                  <Clock className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">Uptime</span>
                </div>
                <div className="text-sm font-mono">
                  {formatUptime(agentStatus.uptime)}
                </div>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* Quick Stats */}
      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <CheckCircle className="h-4 w-4 text-green-500" />
              System Health
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>Agent Service</span>
                <Badge variant={agentStatus?.isRunning ? 'default' : 'secondary'}>
                  {agentStatus?.isRunning ? 'Healthy' : 'Stopped'}
                </Badge>
              </div>
              <div className="flex justify-between text-sm">
                <span>Task Processing</span>
                <Badge variant="default">Normal</Badge>
              </div>
              <div className="flex justify-between text-sm">
                <span>Memory Usage</span>
                <Badge variant="outline">Low</Badge>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Zap className="h-4 w-4 text-yellow-500" />
              Performance
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>Avg Response Time</span>
                <span className="font-mono">124ms</span>
              </div>
              <div className="flex justify-between text-sm">
                <span>Tasks/Hour</span>
                <span className="font-mono">42</span>
              </div>
              <div className="flex justify-between text-sm">
                <span>Error Rate</span>
                <span className="font-mono text-green-600">0.2%</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}