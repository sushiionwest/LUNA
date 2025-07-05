import http from 'k6/http';
import ws from 'k6/ws';
import { check, group, sleep } from 'k6';
import { Counter, Rate, Trend, Gauge } from 'k6/metrics';

// Custom metrics for Luna-specific monitoring
const screenshotRequests = new Counter('luna_screenshot_requests');
const screenshotErrors = new Rate('luna_screenshot_error_rate');
const screenshotDuration = new Trend('luna_screenshot_duration');
const cpuUsage = new Gauge('luna_cpu_usage');
const memoryUsage = new Gauge('luna_memory_usage');
const operationRate = new Gauge('luna_operation_rate');

// Test configuration
export const options = {
  scenarios: {
    screenshot_load: {
      executor: 'constant-rate',
      rate: 10, // 10 screenshots per minute
      timeUnit: '1m',
      duration: '10m', // Run for 10 minutes
      preAllocatedVUs: 2,
      maxVUs: 5,
    },
    ui_automation_load: {
      executor: 'constant-vus',
      vus: 2,
      duration: '10m',
    },
    system_monitoring: {
      executor: 'constant-vus',
      vus: 1,
      duration: '10m',
    }
  },
  thresholds: {
    'luna_screenshot_duration': ['p(95)<5000'], // 95% of screenshots should complete under 5s
    'luna_screenshot_error_rate': ['rate<0.05'], // Less than 5% error rate
    'luna_cpu_usage': ['value<30'], // Average CPU usage should be under 30%
    'luna_memory_usage': ['value<1000000000'], // Memory usage under 1GB
    'http_req_duration': ['p(95)<2000'], // 95% of requests under 2s
    'http_req_failed': ['rate<0.1'], // Less than 10% failed requests
  }
};

// Environment configuration
const BASE_URL = __ENV.LUNA_BASE_URL || 'http://localhost:8080';
const WS_URL = __ENV.LUNA_WS_URL || 'ws://localhost:8080';
const PERFORMANCE_ENDPOINT = `${BASE_URL}/api/performance`;
const SCREENSHOT_ENDPOINT = `${BASE_URL}/api/screenshot`;
const UI_AUTOMATION_ENDPOINT = `${BASE_URL}/api/ui-automation`;

export function setup() {
  console.log('🚀 Starting Luna Performance Load Test');
  console.log(`📊 Target: ${BASE_URL}`);
  console.log(`⏱️  Duration: 10 minutes`);
  console.log(`📸 Screenshot rate: 10/minute`);
  
  // Verify Luna is running
  const healthCheck = http.get(`${BASE_URL}/health`);
  check(healthCheck, {
    'Luna is running': (r) => r.status === 200,
  });

  if (healthCheck.status !== 200) {
    throw new Error('Luna Agent is not responding to health checks');
  }

  return {
    startTime: Date.now(),
    baseUrl: BASE_URL
  };
}

export default function(data) {
  const scenario = __ENV.K6_SCENARIO;
  
  switch (scenario) {
    case 'screenshot_load':
      screenshotLoadTest(data);
      break;
    case 'ui_automation_load':
      uiAutomationLoadTest(data);
      break;
    case 'system_monitoring':
      systemMonitoringTest(data);
      break;
    default:
      // Run all tests in sequence
      screenshotLoadTest(data);
      uiAutomationLoadTest(data);
      systemMonitoringTest(data);
  }
}

function screenshotLoadTest(data) {
  group('Screenshot Load Test', () => {
    const startTime = Date.now();
    
    const response = http.post(SCREENSHOT_ENDPOINT, JSON.stringify({
      format: 'png',
      quality: 85,
      crop: null,
      resize: null
    }), {
      headers: {
        'Content-Type': 'application/json',
      },
      timeout: '30s'
    });

    const duration = Date.now() - startTime;
    screenshotDuration.add(duration);
    screenshotRequests.add(1);

    const success = check(response, {
      'screenshot request successful': (r) => r.status === 200,
      'screenshot response has data': (r) => r.json('filepath') !== undefined,
      'screenshot completed under 10s': (r) => duration < 10000,
    });

    if (!success) {
      screenshotErrors.add(1);
      console.error(`Screenshot request failed: ${response.status} ${response.body}`);
    }

    // Parse performance data from response
    try {
      const responseData = response.json();
      if (responseData.performance) {
        cpuUsage.add(responseData.performance.cpu || 0);
        memoryUsage.add(responseData.performance.memory || 0);
        operationRate.add(responseData.performance.operationsPerMinute || 0);
      }
    } catch (e) {
      // Performance data not available in response
    }

    // Wait for next screenshot (to maintain 10/minute rate)
    sleep(6); // 60 seconds / 10 screenshots = 6 seconds between screenshots
  });
}

function uiAutomationLoadTest(data) {
  group('UI Automation Load Test', () => {
    // Test various UI automation operations
    const operations = [
      { type: 'click', x: 500, y: 300 },
      { type: 'sendkeys', keys: 'Hello World' },
      { type: 'getwindows' },
      { type: 'click', x: 100, y: 100 },
      { type: 'sendkeys', keys: 'Testing UI automation' }
    ];

    operations.forEach((operation, index) => {
      const startTime = Date.now();
      
      const response = http.post(UI_AUTOMATION_ENDPOINT, JSON.stringify(operation), {
        headers: {
          'Content-Type': 'application/json',
        },
        timeout: '10s'
      });

      const duration = Date.now() - startTime;

      check(response, {
        [`${operation.type} operation successful`]: (r) => r.status === 200,
        [`${operation.type} completed under 5s`]: (r) => duration < 5000,
      });

      // Small delay between operations to simulate realistic usage
      sleep(0.5);
    });

    // Longer pause between operation sets
    sleep(10);
  });
}

function systemMonitoringTest(data) {
  group('System Monitoring', () => {
    // Monitor system performance via WebSocket
    const wsUrl = `${WS_URL}/performance-monitor`;
    
    const response = ws.connect(wsUrl, {}, function (socket) {
      socket.on('open', () => {
        console.log('📊 Connected to performance monitoring WebSocket');
        socket.send(JSON.stringify({ action: 'subscribe', metrics: ['cpu', 'memory', 'operations'] }));
      });

      socket.on('message', (data) => {
        try {
          const metrics = JSON.parse(data);
          
          if (metrics.cpu) {
            cpuUsage.add(metrics.cpu.usage);
            
            // Alert if CPU usage is too high
            if (metrics.cpu.usage > 50) {
              console.warn(`⚠️  High CPU usage detected: ${metrics.cpu.usage}%`);
            }
          }

          if (metrics.memory) {
            memoryUsage.add(metrics.memory.used);
            
            // Alert if memory usage is too high
            const memoryUsagePercent = (metrics.memory.used / metrics.memory.total) * 100;
            if (memoryUsagePercent > 80) {
              console.warn(`⚠️  High memory usage detected: ${memoryUsagePercent.toFixed(1)}%`);
            }
          }

          if (metrics.operations) {
            operationRate.add(metrics.operations.totalOperations);
          }

        } catch (e) {
          console.error('Error parsing performance metrics:', e);
        }
      });

      socket.on('error', (e) => {
        console.error('WebSocket error:', e);
      });

      // Keep connection open for monitoring
      sleep(30);
    });

    // Also check HTTP performance endpoint
    const perfResponse = http.get(PERFORMANCE_ENDPOINT);
    check(perfResponse, {
      'performance endpoint accessible': (r) => r.status === 200,
      'performance data valid': (r) => {
        try {
          const data = r.json();
          return data.cpu !== undefined && data.memory !== undefined;
        } catch {
          return false;
        }
      }
    });

    // Log current performance state
    try {
      const perfData = perfResponse.json();
      console.log(`📊 Performance: CPU ${perfData.cpu?.usage || 'N/A'}%, Memory ${perfData.memory?.usage || 'N/A'}%`);
    } catch (e) {
      console.log('📊 Performance data not available');
    }
  });
}

export function teardown(data) {
  console.log('🏁 Luna Performance Load Test Complete');
  
  // Final performance check
  const finalCheck = http.get(`${BASE_URL}/api/performance/summary`);
  if (finalCheck.status === 200) {
    try {
      const summary = finalCheck.json();
      console.log('📈 Final Performance Summary:');
      console.log(`   CPU Average: ${summary.cpu?.average || 'N/A'}%`);
      console.log(`   Memory Peak: ${summary.memory?.peak || 'N/A'} bytes`);
      console.log(`   Total Screenshots: ${summary.operations?.screenshots || 'N/A'}`);
      console.log(`   Total Operations: ${summary.operations?.total || 'N/A'}`);
      console.log(`   Error Rate: ${summary.performance?.errorRate || 'N/A'}%`);
    } catch (e) {
      console.log('📈 Performance summary not available');
    }
  }
  
  // Generate performance report
  const testDuration = (Date.now() - data.startTime) / 1000;
  console.log(`⏱️  Total test duration: ${testDuration} seconds`);
}

// Utility functions for advanced performance testing
export function cpuStressTest() {
  group('CPU Stress Test', () => {
    console.log('🔥 Starting CPU stress test...');
    
    const operations = [];
    for (let i = 0; i < 20; i++) {
      operations.push(
        http.post(SCREENSHOT_ENDPOINT, JSON.stringify({ format: 'png' }), {
          headers: { 'Content-Type': 'application/json' }
        })
      );
    }

    const responses = http.batch(operations);
    
    const successCount = responses.filter(r => r.status === 200).length;
    const avgResponseTime = responses.reduce((sum, r) => sum + r.timings.duration, 0) / responses.length;
    
    console.log(`🔥 CPU Stress Test Results: ${successCount}/${responses.length} successful, avg ${avgResponseTime.toFixed(0)}ms`);
    
    check(responses, {
      'stress test success rate > 80%': () => (successCount / responses.length) > 0.8,
      'stress test avg response < 10s': () => avgResponseTime < 10000,
    });
  });
}

export function memoryLeakTest() {
  group('Memory Leak Test', () => {
    console.log('🧠 Starting memory leak test...');
    
    const initialMemory = http.get(PERFORMANCE_ENDPOINT).json().memory?.used || 0;
    
    // Perform many operations that might cause memory leaks
    for (let i = 0; i < 100; i++) {
      http.post(SCREENSHOT_ENDPOINT, JSON.stringify({ format: 'png' }));
      if (i % 10 === 0) {
        sleep(1); // Small breaks to allow garbage collection
      }
    }
    
    sleep(5); // Allow time for cleanup
    
    const finalMemory = http.get(PERFORMANCE_ENDPOINT).json().memory?.used || 0;
    const memoryIncrease = finalMemory - initialMemory;
    
    console.log(`🧠 Memory Leak Test: Initial ${initialMemory}, Final ${finalMemory}, Increase ${memoryIncrease}`);
    
    check(memoryIncrease, {
      'memory increase < 100MB': (increase) => increase < 100 * 1024 * 1024,
    });
  });
}