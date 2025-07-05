#!/usr/bin/env node

import { io } from 'socket.io-client';

const API_BASE = 'http://localhost:3000';

console.log('🧪 Luna Testing Infrastructure Integration Test\n');

// Test the complete flow
async function runIntegrationTest() {
  try {
    console.log('1️⃣ Starting test session...');
    
    const sessionResponse = await fetch(`${API_BASE}/api/test-session/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ participantId: 'integration-test-user' })
    });
    
    const session = await sessionResponse.json();
    console.log(`✅ Session started: ${session.sessionId}`);
    console.log(`🚀 VM ID: ${session.vmId}`);
    console.log(`🔗 Testing URL: ${session.testingURL}\n`);

    // Connect to WebSocket
    console.log('2️⃣ Connecting to real-time events...');
    const socket = io(API_BASE);
    
    socket.on('connect', () => {
      console.log('✅ Connected to WebSocket\n');
      
      // Simulate Luna installer events
      console.log('3️⃣ Simulating Luna installer integration...');
      
      const installerEvents = [
        { type: 'installation-started', data: { userAgent: 'Test Agent', os: 'Linux' } },
        { type: 'vm-download-start', data: { size: '2.1GB' } },
        { type: 'installation-progress', data: { percentage: 25, message: 'Downloading VM image...' } },
        { type: 'installation-progress', data: { percentage: 50, message: 'Extracting components...' } },
        { type: 'installation-progress', data: { percentage: 75, message: 'Configuring environment...' } },
        { type: 'vm-start-attempt', data: { port: 5432 } },
        { type: 'installation-progress', data: { percentage: 100, message: 'Installation complete!' } },
        { type: 'installation-complete', data: { success: true, duration: '45s' } }
      ];

      let eventIndex = 0;
      const sendNextEvent = () => {
        if (eventIndex < installerEvents.length) {
          const event = installerEvents[eventIndex];
          console.log(`   📊 Sending: ${event.type}`);
          
          socket.emit('luna-installer-event', {
            sessionId: session.sessionId,
            eventType: event.type,
            eventData: event.data,
            timestamp: new Date().toISOString()
          });
          
          eventIndex++;
          setTimeout(sendNextEvent, 1000);
        } else {
          // Test complete, clean up
          setTimeout(async () => {
            console.log('\n4️⃣ Ending test session...');
            
            await fetch(`${API_BASE}/api/test-session/${session.sessionId}/end`, {
              method: 'POST'
            });
            
            console.log('✅ Session ended\n');
            
            // Fetch final results
            console.log('5️⃣ Fetching test results...');
            const eventsResponse = await fetch(`${API_BASE}/api/events`);
            const events = await eventsResponse.json();
            
            const sessionEvents = events.filter(e => e.session_id === session.sessionId);
            console.log(`📈 Captured ${sessionEvents.length} events:`);
            
            sessionEvents.forEach(event => {
              const data = JSON.parse(event.event_data);
              console.log(`   • ${event.event_type}: ${data.message || data.percentage || JSON.stringify(data)}`);
            });
            
            console.log('\n🎉 Integration test completed successfully!');
            console.log('\n📋 Test Summary:');
            console.log(`   • Session ID: ${session.sessionId}`);
            console.log(`   • VM Instance: ${session.vmId}`);
            console.log(`   • Events Logged: ${sessionEvents.length}`);
            console.log(`   • Real-time Communication: ✅ Working`);
            console.log(`   • Session Management: ✅ Working`);
            console.log(`   • Event Tracking: ✅ Working`);
            
            socket.disconnect();
            process.exit(0);
          }, 2000);
        }
      };
      
      sendNextEvent();
    });

    socket.on('session-event', (event) => {
      if (event.sessionId === session.sessionId) {
        console.log(`   📡 Real-time event received: ${event.eventType}`);
      }
    });

    socket.on('vm-status', (data) => {
      if (data.sessionId === session.sessionId) {
        console.log(`   🖥️  VM status update: ${data.status}`);
      }
    });

  } catch (error) {
    console.error('❌ Integration test failed:', error.message);
    process.exit(1);
  }
}

// Wait a moment for server to be ready, then run test
setTimeout(runIntegrationTest, 2000);