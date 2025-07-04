import { useState, useEffect } from 'react';
import { io, Socket } from 'socket.io-client';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { 
  Bot, 
  Activity, 
  Camera, 
  Share2, 
  Settings, 
  Play, 
  Pause, 
  RotateCcw,
  Eye,
  Brain,
  Cpu,
  HardDrive,
  Wifi,
  AlertCircle,
  CheckCircle,
  Clock,
  Zap,
  Download
} from 'lucide-react';

// Components
import { LunaControl } from './components/AgentControl';
import { TaskManager } from './components/TaskManager';
import { ScreenCapture } from './components/ScreenCapture';
import { SocialMedia } from './components/SocialMedia';
import { SystemMonitor } from './components/SystemMonitor';
import { ConfigPanel } from './components/ConfigPanel';
import { ActivityFeed } from './components/ActivityFeed';
import { MetricsChart } from './components/MetricsChart';
import { WindowInstaller } from './components/WindowInstaller';

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

interface SystemHealth {
  agent: AgentStatus;
  screen: {
    isActive: boolean;
    queueLength: number;
  };
  clients: number;
  liveScreenCapture: boolean;
}

export default function Dashboard() {
  const [socket, setSocket] = useState<Socket | null>(null);
  const [connected, setConnected] = useState(false);
  const [agentStatus, setAgentStatus] = useState<AgentStatus | null>(null);
  const [systemHealth, setSystemHealth] = useState<SystemHealth | null>(null);
  const [activeTab, setActiveTab] = useState('overview');

  useEffect(() => {
    // Initialize socket connection
    const newSocket = io('http://localhost:3001');
    
    newSocket.on('connect', () => {
      console.log('Connected to agent server');
      setConnected(true);
    });

    newSocket.on('disconnect', () => {
      console.log('Disconnected from agent server');
      setConnected(false);
    });

    newSocket.on('agent:status', (status: AgentStatus) => {
      setAgentStatus(status);
    });

    newSocket.on('status:data', (data: SystemHealth) => {
      setSystemHealth(data);
    });

    // Request initial status
    newSocket.emit('status:get');

    setSocket(newSocket);

    return () => {
      newSocket.close();
    };
  }, []);

  const startAgent = () => {
    if (socket) {
      socket.emit('agent:start');
    }
  };

  const stopAgent = () => {
    if (socket) {
      socket.emit('agent:stop');
    }
  };

  const getStatusColor = () => {
    if (!connected) return 'bg-red-500';
    if (agentStatus?.isRunning) return 'bg-green-500';
    return 'bg-yellow-500';
  };

  const getStatusText = () => {
    if (!connected) return 'Luna is sleeping';
    if (agentStatus?.isRunning) return 'Luna is watching';
    return 'Luna is ready';
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-background via-background to-muted/20">
      {/* Header */}
      <header className="border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-16 items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="relative">
                <div className="w-8 h-8 rounded-full bg-gradient-to-br from-blue-600 via-purple-600 to-indigo-800 flex items-center justify-center">
                  <span className="text-white font-bold text-sm">🌙</span>
                </div>
              </div>
              <h1 className="text-2xl font-bold bg-gradient-to-r from-blue-600 via-purple-600 to-indigo-800 bg-clip-text text-transparent">
                Luna
              </h1>
            </div>
            <Badge variant="outline" className="ml-4 bg-gradient-to-r from-blue-50 to-purple-50 border-blue-200">
              Your AI Assistant
            </Badge>
          </div>
          
          <div className="flex items-center gap-4">
            {/* Connection Status */}
            <div className="flex items-center gap-2">
              <div className={`h-2 w-2 rounded-full ${getStatusColor()}`} />
              <span className="text-sm font-medium">{getStatusText()}</span>
            </div>
            
            {/* Luna Control */}
            <div className="flex items-center gap-2">
              {agentStatus?.isRunning ? (
                <Button size="sm" variant="outline" onClick={stopAgent}>
                  <Pause className="h-4 w-4 mr-2" />
                  Rest Luna
                </Button>
              ) : (
                <Button size="sm" onClick={startAgent}>
                  <Play className="h-4 w-4 mr-2" />
                  Wake Luna
                </Button>
              )}
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="container py-6">
        <Tabs value={activeTab} onValueChange={setActiveTab} className="space-y-6">
          <TabsList className="grid w-full grid-cols-7">
            <TabsTrigger value="overview" className="flex items-center gap-2">
              <Activity className="h-4 w-4" />
              Overview
            </TabsTrigger>
            <TabsTrigger value="tasks" className="flex items-center gap-2">
              <Brain className="h-4 w-4" />
              Tasks
            </TabsTrigger>
            <TabsTrigger value="screen" className="flex items-center gap-2">
              <Camera className="h-4 w-4" />
              Screen
            </TabsTrigger>
            <TabsTrigger value="social" className="flex items-center gap-2">
              <Share2 className="h-4 w-4" />
              Social
            </TabsTrigger>
            <TabsTrigger value="system" className="flex items-center gap-2">
              <Cpu className="h-4 w-4" />
              System
            </TabsTrigger>
            <TabsTrigger value="installer" className="flex items-center gap-2">
              <Download className="h-4 w-4" />
              Installer
            </TabsTrigger>
            <TabsTrigger value="config" className="flex items-center gap-2">
              <Settings className="h-4 w-4" />
              Config
            </TabsTrigger>
          </TabsList>

          {/* Overview Tab */}
          <TabsContent value="overview" className="space-y-6">
            {/* Status Cards */}
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">Luna Status</CardTitle>
                  <span className="text-lg">🌙</span>
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">
                    {agentStatus?.isRunning ? 'Watching' : 'Ready'}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {agentStatus?.currentTasks || 0} tasks in progress
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">Tasks Processed</CardTitle>
                  <Zap className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{agentStatus?.totalTasksProcessed || 0}</div>
                  <p className="text-xs text-muted-foreground">
                    {agentStatus?.successfulTasks || 0} successful
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">Screen Capture</CardTitle>
                  <Eye className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">
                    {systemHealth?.screen.isActive ? 'Active' : 'Inactive'}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {systemHealth?.screen.queueLength || 0} in queue
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">Connections</CardTitle>
                  <Wifi className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{systemHealth?.clients || 0}</div>
                  <p className="text-xs text-muted-foreground">
                    {connected ? 'Connected' : 'Disconnected'}
                  </p>
                </CardContent>
              </Card>
            </div>

            {/* Main Dashboard Grid */}
            <div className="grid gap-6 lg:grid-cols-3">
              {/* Activity Feed */}
              <div className="lg:col-span-2">
                <ActivityFeed socket={socket} />
              </div>
              
              {/* Quick Actions */}
              <Card>
                <CardHeader>
                  <CardTitle>Quick Actions</CardTitle>
                  <CardDescription>Tell Luna what to do</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <Button className="w-full" onClick={() => setActiveTab('screen')}>
                    <Camera className="h-4 w-4 mr-2" />
                    Ask Luna to Look
                  </Button>
                  <Button variant="outline" className="w-full" onClick={() => setActiveTab('tasks')}>
                    <Brain className="h-4 w-4 mr-2" />
                    Give Luna a Task
                  </Button>
                  <Button variant="outline" className="w-full" onClick={() => setActiveTab('social')}>
                    <Share2 className="h-4 w-4 mr-2" />
                    Luna Social
                  </Button>
                  <Button variant="outline" className="w-full">
                    <RotateCcw className="h-4 w-4 mr-2" />
                    Wake Luna Up
                  </Button>
                </CardContent>
              </Card>
            </div>

            {/* Performance Metrics */}
            <Card>
              <CardHeader>
                <CardTitle>Performance Metrics</CardTitle>
                <CardDescription>Real-time system performance</CardDescription>
              </CardHeader>
              <CardContent>
                <MetricsChart socket={socket} />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Other Tabs */}
          <TabsContent value="overview-control">
            <LunaControl socket={socket} />
          </TabsContent>
          
          <TabsContent value="tasks">
            <TaskManager socket={socket} />
          </TabsContent>

          <TabsContent value="screen">
            <ScreenCapture socket={socket} />
          </TabsContent>

          <TabsContent value="social">
            <SocialMedia socket={socket} />
          </TabsContent>

          <TabsContent value="system">
            <SystemMonitor socket={socket} />
          </TabsContent>

          <TabsContent value="installer">
            <WindowInstaller />
          </TabsContent>

          <TabsContent value="config">
            <ConfigPanel socket={socket} />
          </TabsContent>
        </Tabs>
      </main>
    </div>
  );
}
