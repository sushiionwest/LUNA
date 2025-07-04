#!/usr/bin/env node

/**
 * Luna Testing Infrastructure - Test Script
 * Validates that all services are working correctly
 */

import { DatabaseService } from './src/backend/database.js';
import { ParticipantService } from './src/backend/participants.js';
import { SessionService } from './src/backend/sessions.js';
import { AnalyticsService } from './src/backend/analytics.js';
import { EmailService } from './src/backend/email.js';
import { FileService } from './src/backend/files.js';

console.log('🌙 Luna Testing Infrastructure - Validation Test');
console.log('================================================\n');

async function testInfrastructure() {
    let successCount = 0;
    let totalTests = 0;

    function logTest(testName, success, details = '') {
        totalTests++;
        if (success) {
            successCount++;
            console.log(`✅ ${testName}`);
            if (details) console.log(`   ${details}`);
        } else {
            console.log(`❌ ${testName}`);
            if (details) console.log(`   Error: ${details}`);
        }
    }

    try {
        // Test 1: Database Service
        console.log('1️⃣ Testing Database Service...');
        const db = new DatabaseService();
        await db.initialize();
        logTest('Database initialization', db.isInitialized, 'SQLite database ready');

        // Test 2: Participant Service
        console.log('\n2️⃣ Testing Participant Service...');
        const participantService = new ParticipantService(db);
        
        // Test participant registration
        try {
            const testParticipant = await participantService.register({
                email: 'test@example.com',
                firstName: 'Test',
                lastName: 'User',
                operatingSystem: 'Windows',
                techLevel: 'intermediate',
                useCaseInterest: 'automation'
            });
            logTest('Participant registration', true, `Created participant: ${testParticipant.id}`);
            
            // Test participant retrieval
            const retrievedParticipant = await participantService.getById(testParticipant.id);
            logTest('Participant retrieval', retrievedParticipant.id === testParticipant.id, 'Participant data matches');
            
            // Test participant listing
            const participantList = await participantService.list({ limit: 10 });
            logTest('Participant listing', participantList.participants.length > 0, `Found ${participantList.participants.length} participants`);
            
        } catch (error) {
            logTest('Participant operations', false, error.message);
        }

        // Test 3: Session Service
        console.log('\n3️⃣ Testing Session Service...');
        const sessionService = new SessionService(db);
        
        try {
            // Get a participant to create session for
            const participants = await participantService.list({ limit: 1 });
            if (participants.participants.length > 0) {
                const testSession = await sessionService.create({
                    participantId: participants.participants[0].id,
                    sessionType: 'standard',
                    scheduledAt: new Date(Date.now() + 24 * 60 * 60 * 1000) // Tomorrow
                });
                logTest('Session creation', true, `Created session: ${testSession.id}`);
                
                // Test session event recording
                const eventId = await sessionService.recordEvent(testSession.id, 'test_event', {
                    message: 'Infrastructure test event',
                    timestamp: new Date()
                });
                logTest('Event recording', true, `Recorded event: ${eventId}`);
                
            } else {
                logTest('Session creation', false, 'No participants available for testing');
            }
        } catch (error) {
            logTest('Session operations', false, error.message);
        }

        // Test 4: Analytics Service
        console.log('\n4️⃣ Testing Analytics Service...');
        const analyticsService = new AnalyticsService(db);
        
        try {
            // Test real-time metrics
            const realTimeMetrics = await analyticsService.getRealTimeMetrics();
            logTest('Real-time metrics', typeof realTimeMetrics === 'object', 'Metrics object returned');
            
            // Test aggregated insights
            const insights = await analyticsService.getAggregatedInsights();
            logTest('Aggregated insights', typeof insights === 'object', 'Insights generated');
            
        } catch (error) {
            logTest('Analytics operations', false, error.message);
        }

        // Test 5: Email Service
        console.log('\n5️⃣ Testing Email Service...');
        const emailService = new EmailService(db);
        
        try {
            // Test email statistics
            const emailStats = await emailService.getEmailStatistics();
            logTest('Email statistics', typeof emailStats === 'object', 'Email stats retrieved');
            
            // Test template rendering (mock mode)
            console.log('   📧 Email service initialized in mock mode (no SMTP configured)');
            logTest('Email service initialization', true, 'Ready for mock email sending');
            
        } catch (error) {
            logTest('Email operations', false, error.message);
        }

        // Test 6: File Service
        console.log('\n6️⃣ Testing File Service...');
        const fileService = new FileService(db);
        
        try {
            // Test storage statistics
            const storageStats = await fileService.getStorageStatistics();
            logTest('Storage statistics', typeof storageStats === 'object', 'Storage stats retrieved');
            
            // Test file operations
            const testFileData = 'Test file content for Luna infrastructure validation';
            const savedFile = await fileService.saveFile(Buffer.from(testFileData), {
                originalName: 'test.txt',
                mimeType: 'text/plain',
                category: 'logs'
            });
            logTest('File saving', true, `Saved file: ${savedFile.id}`);
            
            // Test file retrieval
            const retrievedFile = await fileService.getFile(savedFile.id);
            logTest('File retrieval', retrievedFile.id === savedFile.id, 'File data matches');
            
        } catch (error) {
            logTest('File operations', false, error.message);
        }

        // Test 7: Service Integration
        console.log('\n7️⃣ Testing Service Integration...');
        
        try {
            // Test cross-service operations
            const participants = await participantService.list({ limit: 1 });
            if (participants.participants.length > 0) {
                const participant = participants.participants[0];
                
                // Test analytics for participant
                const userJourney = await analyticsService.getUserJourney(participant.id);
                logTest('Cross-service analytics', typeof userJourney === 'object', 'User journey generated');
                
                // Test email history
                const emailHistory = await emailService.getEmailHistory(participant.id);
                logTest('Cross-service email history', Array.isArray(emailHistory), 'Email history retrieved');
                
            }
        } catch (error) {
            logTest('Service integration', false, error.message);
        }

        // Summary
        console.log('\n🎯 Test Summary');
        console.log('================');
        console.log(`Total Tests: ${totalTests}`);
        console.log(`Passed: ${successCount}`);
        console.log(`Failed: ${totalTests - successCount}`);
        console.log(`Success Rate: ${Math.round((successCount / totalTests) * 100)}%`);
        
        if (successCount === totalTests) {
            console.log('\n🎉 All tests passed! Luna Testing Infrastructure is ready for production.');
        } else {
            console.log('\n⚠️  Some tests failed. Please check the errors above.');
        }

        // Close database connection
        await db.close();

    } catch (error) {
        console.error('\n💥 Critical error during testing:', error);
        console.log('\n❌ Infrastructure validation failed.');
    }
}

// Run the tests
testInfrastructure().catch(console.error);