import { useState, useEffect } from 'react';
import { Socket } from 'socket.io-client';
import { Line, LineChart, ResponsiveContainer, XAxis, YAxis, CartesianGrid, Tooltip, Legend } from 'recharts';

interface MetricsChartProps {
  socket: Socket | null;
}

interface MetricData {
  timestamp: string;
  tasksPerMinute: number;
  successRate: number;
  responseTime: number;
  memoryUsage: number;
  cpuUsage: number;
}

export function MetricsChart({ socket }: MetricsChartProps) {
  const [metrics, setMetrics] = useState<MetricData[]>([]);

  useEffect(() => {
    // Generate initial mock data
    const now = new Date();
    const initialMetrics: MetricData[] = [];
    
    for (let i = 29; i >= 0; i--) {
      const timestamp = new Date(now.getTime() - i * 60000);
      initialMetrics.push({
        timestamp: timestamp.toLocaleTimeString('en-US', { 
          hour12: false, 
          hour: '2-digit', 
          minute: '2-digit' 
        }),
        tasksPerMinute: Math.floor(Math.random() * 10) + 5,
        successRate: Math.floor(Math.random() * 20) + 80,
        responseTime: Math.floor(Math.random() * 100) + 50,
        memoryUsage: Math.floor(Math.random() * 30) + 40,
        cpuUsage: Math.floor(Math.random() * 40) + 20
      });
    }
    
    setMetrics(initialMetrics);

    if (!socket) return;

    // Update metrics every minute
    const interval = setInterval(() => {
      const newMetric: MetricData = {
        timestamp: new Date().toLocaleTimeString('en-US', { 
          hour12: false, 
          hour: '2-digit', 
          minute: '2-digit' 
        }),
        tasksPerMinute: Math.floor(Math.random() * 10) + 5,
        successRate: Math.floor(Math.random() * 20) + 80,
        responseTime: Math.floor(Math.random() * 100) + 50,
        memoryUsage: Math.floor(Math.random() * 30) + 40,
        cpuUsage: Math.floor(Math.random() * 40) + 20
      };

      setMetrics(prev => [...prev.slice(1), newMetric]);
    }, 60000);

    return () => clearInterval(interval);
  }, [socket]);

  const formatTooltipLabel = (label: string) => {
    return `Time: ${label}`;
  };

  const formatTooltipValue = (value: number, name: string) => {
    switch (name) {
      case 'tasksPerMinute':
        return [`${value} tasks`, 'Tasks/Min'];
      case 'successRate':
        return [`${value}%`, 'Success Rate'];
      case 'responseTime':
        return [`${value}ms`, 'Response Time'];
      case 'memoryUsage':
        return [`${value}%`, 'Memory Usage'];
      case 'cpuUsage':
        return [`${value}%`, 'CPU Usage'];
      default:
        return [value, name];
    }
  };

  return (
    <div className="h-80">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={metrics}
          margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
        >
          <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
          <XAxis 
            dataKey="timestamp" 
            tick={{ fontSize: 12 }}
            interval="preserveStartEnd"
          />
          <YAxis tick={{ fontSize: 12 }} />
          <Tooltip 
            labelFormatter={formatTooltipLabel}
            formatter={formatTooltipValue}
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '6px'
            }}
          />
          <Legend />
          <Line
            type="monotone"
            dataKey="tasksPerMinute"
            stroke="hsl(var(--primary))"
            strokeWidth={2}
            dot={{ r: 3 }}
            name="Tasks/Min"
          />
          <Line
            type="monotone"
            dataKey="successRate"
            stroke="hsl(142 76% 36%)"
            strokeWidth={2}
            dot={{ r: 3 }}
            name="Success Rate (%)"
          />
          <Line
            type="monotone"
            dataKey="responseTime"
            stroke="hsl(48 96% 53%)"
            strokeWidth={2}
            dot={{ r: 3 }}
            name="Response Time (ms)"
          />
          <Line
            type="monotone"
            dataKey="memoryUsage"
            stroke="hsl(263 70% 50%)"
            strokeWidth={2}
            dot={{ r: 3 }}
            name="Memory (%)"
          />
          <Line
            type="monotone"
            dataKey="cpuUsage"
            stroke="hsl(0 84% 60%)"
            strokeWidth={2}
            dot={{ r: 3 }}
            name="CPU (%)"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}