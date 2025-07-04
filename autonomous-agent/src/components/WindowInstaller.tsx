import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Progress } from '@/components/ui/progress';
import { 
  Download, 
  Package, 
  Trash2, 
  Search, 
  Filter, 
  Monitor, 
  Minimize, 
  Maximize, 
  X,
  Play,
  Pause,
  RotateCcw,
  Settings,
  CheckCircle,
  AlertCircle,
  Clock,
  HardDrive,
  Cpu,
  Memory
} from 'lucide-react';

interface InstallationPackage {
  id: string;
  name: string;
  version: string;
  description: string;
  category: string;
  platform: string;
  architecture: string;
  fileSize: number;
  installationType: string;
}

interface InstallationTask {
  id: string;
  packageId: string;
  status: 'pending' | 'downloading' | 'installing' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  startTime: string;
  endTime?: string;
  errorMessage?: string;
}

interface InstalledApplication {
  id: string;
  packageId: string;
  name: string;
  version: string;
  installPath: string;
  installDate: string;
  size: number;
  autoStart: boolean;
}

interface WindowManager {
  id: string;
  applicationId: string;
  windowTitle: string;
  processId: number;
  position: { x: number; y: number };
  size: { width: number; height: number };
  state: string;
  isActive: boolean;
  workspace?: string;
}

export const WindowInstaller: React.FC = () => {
  const [packages, setPackages] = useState<InstallationPackage[]>([]);
  const [installedApps, setInstalledApps] = useState<InstalledApplication[]>([]);
  const [installations, setInstallations] = useState<InstallationTask[]>([]);
  const [windows, setWindows] = useState<WindowManager[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [systemInfo, setSystemInfo] = useState<any>(null);

  // Fetch data on component mount
  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, []);

  const fetchData = async () => {
    try {
      await Promise.all([
        fetchPackages(),
        fetchInstalledApps(),
        fetchInstallations(),
        fetchWindows(),
        fetchSystemInfo()
      ]);
    } catch (error) {
      console.error('Error fetching data:', error);
    } finally {
      setLoading(false);
    }
  };

  const fetchPackages = async () => {
    try {
      const response = await fetch('/api/installer/packages');
      const data = await response.json();
      if (data.success) {
        setPackages(data.packages);
      }
    } catch (error) {
      console.error('Error fetching packages:', error);
    }
  };

  const fetchInstalledApps = async () => {
    try {
      const response = await fetch('/api/installer/installed');
      const data = await response.json();
      if (data.success) {
        setInstalledApps(data.applications);
      }
    } catch (error) {
      console.error('Error fetching installed apps:', error);
    }
  };

  const fetchInstallations = async () => {
    try {
      const response = await fetch('/api/installer/installations');
      const data = await response.json();
      if (data.success) {
        setInstallations(data.installations);
      }
    } catch (error) {
      console.error('Error fetching installations:', error);
    }
  };

  const fetchWindows = async () => {
    try {
      const response = await fetch('/api/installer/windows');
      const data = await response.json();
      if (data.success) {
        setWindows(data.windows);
      }
    } catch (error) {
      console.error('Error fetching windows:', error);
    }
  };

  const fetchSystemInfo = async () => {
    try {
      const response = await fetch('/api/installer/system/info');
      const data = await response.json();
      if (data.success) {
        setSystemInfo(data.systemInfo);
      }
    } catch (error) {
      console.error('Error fetching system info:', error);
    }
  };

  const handleInstallPackage = async (packageId: string, options = {}) => {
    try {
      const response = await fetch('/api/installer/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ packageId, options })
      });
      const data = await response.json();
      if (data.success) {
        await fetchInstallations();
      }
    } catch (error) {
      console.error('Error installing package:', error);
    }
  };

  const handleUninstallApp = async (appId: string) => {
    try {
      const response = await fetch(`/api/installer/installed/${appId}`, {
        method: 'DELETE'
      });
      const data = await response.json();
      if (data.success) {
        await fetchInstalledApps();
      }
    } catch (error) {
      console.error('Error uninstalling app:', error);
    }
  };

  const handleCancelInstallation = async (taskId: string) => {
    try {
      const response = await fetch(`/api/installer/install/${taskId}/cancel`, {
        method: 'POST'
      });
      const data = await response.json();
      if (data.success) {
        await fetchInstallations();
      }
    } catch (error) {
      console.error('Error cancelling installation:', error);
    }
  };

  const handleManageWindow = async (windowId: string, action: string) => {
    try {
      const response = await fetch(`/api/installer/windows/${windowId}/${action}`, {
        method: 'POST'
      });
      const data = await response.json();
      if (data.success) {
        await fetchWindows();
      }
    } catch (error) {
      console.error('Error managing window:', error);
    }
  };

  const handleSearchPackages = async (query: string) => {
    try {
      const response = await fetch(`/api/installer/packages/search?query=${encodeURIComponent(query)}`);
      const data = await response.json();
      if (data.success) {
        setPackages(data.packages);
      }
    } catch (error) {
      console.error('Error searching packages:', error);
    }
  };

  const filteredPackages = packages.filter(pkg => {
    const matchesSearch = pkg.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
                         pkg.description.toLowerCase().includes(searchTerm.toLowerCase());
    const matchesCategory = selectedCategory === 'all' || pkg.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const categories = ['all', ...Array.from(new Set(packages.map(pkg => pkg.category)))];

  const formatFileSize = (bytes: number): string => {
    const sizes = ['B', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 B';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${Math.round(bytes / Math.pow(1024, i) * 100) / 100} ${sizes[i]}`;
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'failed':
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      case 'pending':
      case 'downloading':
      case 'installing':
        return <Clock className="w-4 h-4 text-blue-500" />;
      case 'cancelled':
        return <X className="w-4 h-4 text-gray-500" />;
      default:
        return <Clock className="w-4 h-4 text-gray-500" />;
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">Window Installer & Manager</h2>
        <Button onClick={fetchData} variant="outline" size="sm">
          <RotateCcw className="w-4 h-4 mr-2" />
          Refresh
        </Button>
      </div>

      <Tabs defaultValue="packages" className="w-full">
        <TabsList className="grid w-full grid-cols-5">
          <TabsTrigger value="packages">Available Packages</TabsTrigger>
          <TabsTrigger value="installed">Installed Apps</TabsTrigger>
          <TabsTrigger value="installations">Installations</TabsTrigger>
          <TabsTrigger value="windows">Window Manager</TabsTrigger>
          <TabsTrigger value="system">System Info</TabsTrigger>
        </TabsList>

        <TabsContent value="packages" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Package className="w-5 h-5" />
                Available Packages
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex gap-4 mb-4">
                <div className="flex-1 relative">
                  <Search className="absolute left-3 top-3 h-4 w-4 text-gray-400" />
                  <Input
                    placeholder="Search packages..."
                    value={searchTerm}
                    onChange={(e) => setSearchTerm(e.target.value)}
                    className="pl-10"
                  />
                </div>
                <div className="flex items-center gap-2">
                  <Filter className="w-4 h-4" />
                  <select 
                    value={selectedCategory} 
                    onChange={(e) => setSelectedCategory(e.target.value)}
                    className="px-3 py-2 border rounded-md"
                  >
                    {categories.map(cat => (
                      <option key={cat} value={cat}>
                        {cat === 'all' ? 'All Categories' : cat.charAt(0).toUpperCase() + cat.slice(1)}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {filteredPackages.map((pkg) => (
                  <Card key={pkg.id} className="transition-all hover:shadow-lg">
                    <CardHeader className="pb-3">
                      <div className="flex justify-between items-start">
                        <div>
                          <h3 className="font-semibold">{pkg.name}</h3>
                          <p className="text-sm text-gray-600">v{pkg.version}</p>
                        </div>
                        <Badge variant="secondary">{pkg.category}</Badge>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <p className="text-sm text-gray-600 mb-3">{pkg.description}</p>
                      <div className="space-y-2 mb-4">
                        <div className="flex justify-between text-xs">
                          <span>Size:</span>
                          <span>{formatFileSize(pkg.fileSize)}</span>
                        </div>
                        <div className="flex justify-between text-xs">
                          <span>Type:</span>
                          <span className="uppercase">{pkg.installationType}</span>
                        </div>
                        <div className="flex justify-between text-xs">
                          <span>Platform:</span>
                          <span>{pkg.platform} ({pkg.architecture})</span>
                        </div>
                      </div>
                      <Button 
                        onClick={() => handleInstallPackage(pkg.id)}
                        className="w-full"
                        size="sm"
                      >
                        <Download className="w-4 h-4 mr-2" />
                        Install
                      </Button>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="installed" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <HardDrive className="w-5 h-5" />
                Installed Applications ({installedApps.length})
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {installedApps.map((app) => (
                  <div key={app.id} className="flex items-center justify-between p-4 border rounded-lg">
                    <div>
                      <h3 className="font-semibold">{app.name}</h3>
                      <p className="text-sm text-gray-600">v{app.version}</p>
                      <p className="text-xs text-gray-500">
                        Installed: {new Date(app.installDate).toLocaleDateString()}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant={app.autoStart ? "default" : "secondary"}>
                        {app.autoStart ? "Auto-start" : "Manual"}
                      </Badge>
                      <Button
                        onClick={() => handleUninstallApp(app.id)}
                        variant="destructive"
                        size="sm"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="installations" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Clock className="w-5 h-5" />
                Installation Progress
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {installations.map((task) => (
                  <div key={task.id} className="p-4 border rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        {getStatusIcon(task.status)}
                        <h3 className="font-semibold">{task.packageId}</h3>
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant={
                          task.status === 'completed' ? 'default' :
                          task.status === 'failed' ? 'destructive' :
                          'secondary'
                        }>
                          {task.status}
                        </Badge>
                        {task.status !== 'completed' && task.status !== 'failed' && (
                          <Button
                            onClick={() => handleCancelInstallation(task.id)}
                            variant="outline"
                            size="sm"
                          >
                            Cancel
                          </Button>
                        )}
                      </div>
                    </div>
                    
                    {task.status !== 'completed' && task.status !== 'failed' && (
                      <div className="mb-2">
                        <div className="flex justify-between text-sm mb-1">
                          <span>Progress</span>
                          <span>{task.progress}%</span>
                        </div>
                        <Progress value={task.progress} className="h-2" />
                      </div>
                    )}
                    
                    <div className="text-xs text-gray-500">
                      Started: {new Date(task.startTime).toLocaleString()}
                      {task.endTime && (
                        <span className="ml-4">
                          Ended: {new Date(task.endTime).toLocaleString()}
                        </span>
                      )}
                    </div>
                    
                    {task.errorMessage && (
                      <div className="mt-2 p-2 bg-red-50 border border-red-200 rounded text-sm text-red-700">
                        {task.errorMessage}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="windows" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Monitor className="w-5 h-5" />
                Window Manager ({windows.length} windows)
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {windows.map((window) => (
                  <div key={window.id} className="p-4 border rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <div>
                        <h3 className="font-semibold">{window.windowTitle}</h3>
                        <p className="text-sm text-gray-600">
                          PID: {window.processId} | Position: {window.position.x}, {window.position.y}
                        </p>
                        <p className="text-xs text-gray-500">
                          Size: {window.size.width}x{window.size.height} | State: {window.state}
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant={window.isActive ? "default" : "secondary"}>
                          {window.isActive ? "Active" : "Inactive"}
                        </Badge>
                        <div className="flex gap-1">
                          <Button
                            onClick={() => handleManageWindow(window.id, 'focus')}
                            variant="outline"
                            size="sm"
                          >
                            <Play className="w-4 h-4" />
                          </Button>
                          <Button
                            onClick={() => handleManageWindow(window.id, 'minimize')}
                            variant="outline"
                            size="sm"
                          >
                            <Minimize className="w-4 h-4" />
                          </Button>
                          <Button
                            onClick={() => handleManageWindow(window.id, 'maximize')}
                            variant="outline"
                            size="sm"
                          >
                            <Maximize className="w-4 h-4" />
                          </Button>
                          <Button
                            onClick={() => handleManageWindow(window.id, 'close')}
                            variant="destructive"
                            size="sm"
                          >
                            <X className="w-4 h-4" />
                          </Button>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="system" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Settings className="w-5 h-5" />
                System Information
              </CardTitle>
            </CardHeader>
            <CardContent>
              {systemInfo && (
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <h4 className="font-semibold flex items-center gap-2">
                      <Cpu className="w-4 h-4" />
                      System Details
                    </h4>
                    <div className="space-y-1 text-sm">
                      <div className="flex justify-between">
                        <span>OS:</span>
                        <span>{systemInfo.os}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Platform:</span>
                        <span>{systemInfo.platform}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Architecture:</span>
                        <span>{systemInfo.arch}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Kernel:</span>
                        <span>{systemInfo.kernel}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Node.js:</span>
                        <span>{systemInfo.nodeVersion}</span>
                      </div>
                    </div>
                  </div>
                  
                  <div className="space-y-2">
                    <h4 className="font-semibold flex items-center gap-2">
                      <Memory className="w-4 h-4" />
                      Resources
                    </h4>
                    <div className="space-y-1 text-sm">
                      <div className="flex justify-between">
                        <span>Available Space:</span>
                        <span>{systemInfo.availableSpace}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Memory Usage:</span>
                        <span>{formatFileSize(systemInfo.memory.rss)}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Uptime:</span>
                        <span>{Math.floor(systemInfo.uptime / 3600)}h {Math.floor((systemInfo.uptime % 3600) / 60)}m</span>
                      </div>
                    </div>
                  </div>
                  
                  <div className="space-y-2">
                    <h4 className="font-semibold">Package Managers</h4>
                    <div className="flex flex-wrap gap-2">
                      {Object.entries(systemInfo.packageManagers).map(([pm, available]) => (
                        <Badge key={pm} variant={available ? "default" : "secondary"}>
                          {pm.toUpperCase()}
                        </Badge>
                      ))}
                    </div>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
};