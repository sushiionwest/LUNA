/**
 * Email Service - Handles automated participant communication and notifications
 */

import nodemailer from 'nodemailer';
import { v4 as uuidv4 } from 'uuid';

export class EmailService {
    constructor(database, config = {}) {
        this.db = database;
        this.config = {
            smtpHost: config.smtpHost || process.env.SMTP_HOST || 'smtp.gmail.com',
            smtpPort: config.smtpPort || process.env.SMTP_PORT || 587,
            smtpUser: config.smtpUser || process.env.SMTP_USER,
            smtpPass: config.smtpPass || process.env.SMTP_PASS,
            fromEmail: config.fromEmail || process.env.FROM_EMAIL || 'luna-testing@yourcompany.com',
            fromName: config.fromName || 'Luna Testing Team',
            replyTo: config.replyTo || process.env.REPLY_TO_EMAIL,
            ...config
        };

        this.transporter = null;
        this.emailQueue = [];
        this.isProcessingQueue = false;
        this.templates = new Map();
        
        this.initializeTransporter();
        this.loadEmailTemplates();
        this.startQueueProcessor();
    }

    async initializeTransporter() {
        try {
            this.transporter = nodemailer.createTransporter({
                host: this.config.smtpHost,
                port: this.config.smtpPort,
                secure: this.config.smtpPort === 465, // true for 465, false for other ports
                auth: {
                    user: this.config.smtpUser,
                    pass: this.config.smtpPass
                },
                tls: {
                    rejectUnauthorized: false // Allow self-signed certificates in development
                }
            });

            // Verify connection
            if (this.config.smtpUser && this.config.smtpPass) {
                await this.transporter.verify();
                console.log('Email service initialized successfully');
            } else {
                console.log('Email service initialized in mock mode (no credentials provided)');
            }

        } catch (error) {
            console.error('Email service initialization error:', error);
            this.transporter = null; // Fall back to mock mode
        }
    }

    loadEmailTemplates() {
        // Welcome email template
        this.templates.set('welcome', {
            subject: 'Welcome to Luna User Testing!',
            html: `
                <div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto;">
                    <div style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; text-align: center;">
                        <h1 style="margin: 0; font-size: 28px;">Welcome to Luna Testing! 🌙</h1>
                        <p style="margin: 10px 0 0 0; font-size: 16px; opacity: 0.9;">Thank you for joining our user research program</p>
                    </div>
                    <div style="padding: 30px; background: #f8f9fa;">
                        <p>Hi {{firstName}},</p>
                        <p>Welcome to the Luna user testing program! We're excited to have you help us improve Luna's one-click installation experience.</p>
                        
                        <h3 style="color: #667eea;">What happens next?</h3>
                        <ul style="line-height: 1.6;">
                            <li><strong>Screening:</strong> We'll review your application and may contact you with a few quick questions</li>
                            <li><strong>Scheduling:</strong> If selected, we'll send you a calendar link to book your testing session</li>
                            <li><strong>Testing:</strong> Join us for a 30-45 minute remote session where you'll try Luna for the first time</li>
                            <li><strong>Feedback:</strong> Share your thoughts and help us make Luna better for everyone</li>
                        </ul>

                        <div style="background: #e3f2fd; padding: 20px; border-radius: 8px; margin: 20px 0;">
                            <h4 style="margin-top: 0; color: #1976d2;">Your Registration Details</h4>
                            <p><strong>Email:</strong> {{email}}</p>
                            <p><strong>Tech Level:</strong> {{techLevel}}</p>
                            <p><strong>Operating System:</strong> {{operatingSystem}}</p>
                            <p><strong>Participant ID:</strong> {{participantId}}</p>
                        </div>

                        <h3 style="color: #667eea;">Questions?</h3>
                        <p>If you have any questions or need to update your information, just reply to this email. We're here to help!</p>
                        
                        <p>Thanks again for your interest in Luna!</p>
                        <p><strong>The Luna Testing Team</strong></p>
                    </div>
                    <div style="background: #263238; color: #90a4ae; padding: 20px; text-align: center; font-size: 12px;">
                        <p>Luna User Testing Program | {{replyTo}}</p>
                        <p>You received this email because you signed up for Luna user testing.</p>
                    </div>
                </div>
            `,
            text: `
Welcome to Luna Testing!

Hi {{firstName}},

Welcome to the Luna user testing program! We're excited to have you help us improve Luna's one-click installation experience.

What happens next?
- Screening: We'll review your application and may contact you with a few quick questions
- Scheduling: If selected, we'll send you a calendar link to book your testing session  
- Testing: Join us for a 30-45 minute remote session where you'll try Luna for the first time
- Feedback: Share your thoughts and help us make Luna better for everyone

Your Registration Details:
- Email: {{email}}
- Tech Level: {{techLevel}}
- Operating System: {{operatingSystem}}
- Participant ID: {{participantId}}

Questions?
If you have any questions or need to update your information, just reply to this email.

Thanks again for your interest in Luna!
The Luna Testing Team
            `
        });

        // Session reminder template
        this.templates.set('session_reminder', {
            subject: 'Your Luna Testing Session is Tomorrow',
            html: `
                <div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto;">
                    <div style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; text-align: center;">
                        <h1 style="margin: 0; font-size: 28px;">Testing Session Reminder 🕐</h1>
                        <p style="margin: 10px 0 0 0; font-size: 16px; opacity: 0.9;">Your Luna session is coming up!</p>
                    </div>
                    <div style="padding: 30px; background: #f8f9fa;">
                        <p>Hi {{firstName}},</p>
                        <p>This is a friendly reminder that your Luna testing session is scheduled for tomorrow.</p>
                        
                        <div style="background: #e8f5e8; padding: 20px; border-radius: 8px; margin: 20px 0; border-left: 4px solid #4caf50;">
                            <h3 style="margin-top: 0; color: #2e7d32;">Session Details</h3>
                            <p><strong>Date & Time:</strong> {{sessionDate}} at {{sessionTime}}</p>
                            <p><strong>Duration:</strong> {{estimatedDuration}} minutes</p>
                            <p><strong>Meeting Link:</strong> <a href="{{meetingLink}}" style="color: #1976d2;">{{meetingLink}}</a></p>
                            <p><strong>Session ID:</strong> {{sessionId}}</p>
                        </div>

                        <h3 style="color: #667eea;">What to Expect</h3>
                        <ul style="line-height: 1.6;">
                            <li>We'll start with a brief introduction and overview</li>
                            <li>You'll download and install Luna while thinking aloud</li>
                            <li>We'll ask you to try a few simple tasks</li>
                            <li>We'll wrap up with questions about your experience</li>
                        </ul>

                        <h3 style="color: #667eea;">How to Prepare</h3>
                        <ul style="line-height: 1.6;">
                            <li>Ensure you have a stable internet connection</li>
                            <li>Use your {{operatingSystem}} computer</li>
                            <li>Have administrator access ready (you may need to install software)</li>
                            <li>Find a quiet space where you can speak freely</li>
                            <li>Test your microphone and camera beforehand</li>
                        </ul>

                        <div style="background: #fff3e0; padding: 20px; border-radius: 8px; margin: 20px 0; border-left: 4px solid #ff9800;">
                            <h4 style="margin-top: 0; color: #ef6c00;">Need to Reschedule?</h4>
                            <p>If you can't make it tomorrow, please reply to this email as soon as possible so we can find a new time that works.</p>
                        </div>

                        <p>We're looking forward to seeing you tomorrow and learning from your experience with Luna!</p>
                        <p><strong>The Luna Testing Team</strong></p>
                    </div>
                </div>
            `,
            text: `
Session Reminder

Hi {{firstName}},

This is a friendly reminder that your Luna testing session is scheduled for tomorrow.

Session Details:
- Date & Time: {{sessionDate}} at {{sessionTime}}
- Duration: {{estimatedDuration}} minutes  
- Meeting Link: {{meetingLink}}
- Session ID: {{sessionId}}

What to Expect:
- Brief introduction and overview
- Download and install Luna while thinking aloud
- Try a few simple tasks
- Questions about your experience

How to Prepare:
- Ensure stable internet connection
- Use your {{operatingSystem}} computer
- Have administrator access ready
- Find a quiet space
- Test your microphone and camera

Need to reschedule? Reply to this email ASAP.

Looking forward to tomorrow!
The Luna Testing Team
            `
        });

        // Session confirmation template
        this.templates.set('session_scheduled', {
            subject: 'Your Luna Testing Session is Confirmed!',
            html: `
                <div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto;">
                    <div style="background: linear-gradient(135deg, #4caf50 0%, #45a049 100%); color: white; padding: 30px; text-align: center;">
                        <h1 style="margin: 0; font-size: 28px;">Session Confirmed! ✅</h1>
                        <p style="margin: 10px 0 0 0; font-size: 16px; opacity: 0.9;">You're all set for Luna testing</p>
                    </div>
                    <div style="padding: 30px; background: #f8f9fa;">
                        <p>Hi {{firstName}},</p>
                        <p>Great news! Your Luna testing session has been confirmed. We can't wait to see what you think of our one-click installation experience.</p>
                        
                        <div style="background: #e8f5e8; padding: 20px; border-radius: 8px; margin: 20px 0; border-left: 4px solid #4caf50;">
                            <h3 style="margin-top: 0; color: #2e7d32;">Your Session</h3>
                            <p><strong>Date & Time:</strong> {{sessionDate}} at {{sessionTime}}</p>
                            <p><strong>Duration:</strong> {{estimatedDuration}} minutes</p>
                            <p><strong>Meeting Link:</strong> <a href="{{meetingLink}}" style="color: #1976d2;">{{meetingLink}}</a></p>
                            <p><strong>Facilitator:</strong> {{facilitatorName}}</p>
                        </div>

                        <div style="text-align: center; margin: 30px 0;">
                            <a href="{{calendarLink}}" style="background: #667eea; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">📅 Add to Calendar</a>
                        </div>

                        <p>We'll send you a reminder email the day before your session with all the details and preparation tips.</p>
                        
                        <p>If you need to reschedule or have any questions, just reply to this email.</p>
                        <p><strong>The Luna Testing Team</strong></p>
                    </div>
                </div>
            `
        });

        // Thank you email template
        this.templates.set('thank_you', {
            subject: 'Thank You for Testing Luna! 🌟',
            html: `
                <div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto;">
                    <div style="background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%); color: white; padding: 30px; text-align: center;">
                        <h1 style="margin: 0; font-size: 28px;">Thank You! 🌟</h1>
                        <p style="margin: 10px 0 0 0; font-size: 16px; opacity: 0.9;">Your feedback is incredibly valuable</p>
                    </div>
                    <div style="padding: 30px; background: #f8f9fa;">
                        <p>Hi {{firstName}},</p>
                        <p>Thank you for participating in our Luna testing session! Your insights and feedback are incredibly valuable and will help us make Luna better for everyone.</p>
                        
                        <div style="background: #e3f2fd; padding: 20px; border-radius: 8px; margin: 20px 0;">
                            <h3 style="margin-top: 0; color: #1976d2;">Your Session Summary</h3>
                            <p><strong>Session Date:</strong> {{sessionDate}}</p>
                            <p><strong>Duration:</strong> {{actualDuration}} minutes</p>
                            <p><strong>Tasks Completed:</strong> {{completedTasks}}/{{totalTasks}}</p>
                            {{#if completionScore}}<p><strong>Completion Score:</strong> {{completionScore}}%</p>{{/if}}
                        </div>

                        <h3 style="color: #667eea;">What's Next?</h3>
                        <ul style="line-height: 1.6;">
                            <li>We'll analyze your session data and feedback</li>
                            <li>Your insights will be incorporated into Luna's development</li>
                            <li>We may reach out for follow-up questions (optional)</li>
                            <li>You'll be notified when Luna is publicly available</li>
                        </ul>

                        {{#if incentiveInfo}}
                        <div style="background: #e8f5e8; padding: 20px; border-radius: 8px; margin: 20px 0;">
                            <h3 style="margin-top: 0; color: #2e7d32;">Thank You Gift</h3>
                            <p>{{incentiveInfo}}</p>
                        </div>
                        {{/if}}

                        <p>If you have any additional thoughts or questions about Luna, feel free to reply to this email. We love hearing from our testers!</p>
                        
                        <p>Thank you again for your time and valuable feedback!</p>
                        <p><strong>The Luna Testing Team</strong></p>
                    </div>
                </div>
            `
        });

        // Follow-up feedback request
        this.templates.set('feedback_request', {
            subject: 'Quick Follow-up: How was your Luna experience?',
            html: `
                <div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto;">
                    <div style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; text-align: center;">
                        <h1 style="margin: 0; font-size: 28px;">How Did We Do? 💭</h1>
                        <p style="margin: 10px 0 0 0; font-size: 16px; opacity: 0.9;">Your additional feedback helps us improve</p>
                    </div>
                    <div style="padding: 30px; background: #f8f9fa;">
                        <p>Hi {{firstName}},</p>
                        <p>It's been a few days since your Luna testing session, and we'd love to hear any additional thoughts you might have.</p>
                        
                        <div style="text-align: center; margin: 30px 0;">
                            <a href="{{feedbackLink}}" style="background: #667eea; color: white; padding: 15px 30px; text-decoration: none; border-radius: 6px; display: inline-block; font-size: 16px;">Share Your Thoughts</a>
                        </div>

                        <p>The quick survey takes just 2-3 minutes and covers:</p>
                        <ul style="line-height: 1.6;">
                            <li>Overall satisfaction with the testing experience</li>
                            <li>How likely you are to recommend Luna to others</li>
                            <li>Any additional suggestions for improvement</li>
                        </ul>

                        <p>Your feedback is completely optional, but incredibly helpful for making Luna even better.</p>
                        
                        <p>Thanks again!</p>
                        <p><strong>The Luna Testing Team</strong></p>
                    </div>
                </div>
            `
        });

        console.log(`Loaded ${this.templates.size} email templates`);
    }

    async sendEmail(to, templateName, variables = {}, options = {}) {
        try {
            const emailId = uuidv4();
            
            // Get template
            const template = this.templates.get(templateName);
            if (!template) {
                throw new Error(`Email template '${templateName}' not found`);
            }

            // Merge default variables
            const mergedVariables = {
                replyTo: this.config.replyTo,
                fromName: this.config.fromName,
                ...variables
            };

            // Render template
            const renderedEmail = this.renderTemplate(template, mergedVariables);

            // Prepare email data
            const emailData = {
                id: emailId,
                to: to,
                subject: renderedEmail.subject,
                html: renderedEmail.html,
                text: renderedEmail.text || null,
                template: templateName,
                variables: mergedVariables,
                priority: options.priority || 'normal',
                scheduledFor: options.scheduledFor || null,
                retryCount: 0,
                createdAt: new Date()
            };

            // Add to queue
            this.emailQueue.push(emailData);

            // Log email creation
            await this.logEmailEvent('email_queued', emailId, {
                to: to,
                template: templateName,
                priority: emailData.priority
            });

            // Process queue if not already processing
            if (!this.isProcessingQueue) {
                this.processEmailQueue();
            }

            return emailId;

        } catch (error) {
            console.error('Send email error:', error);
            throw error;
        }
    }

    renderTemplate(template, variables) {
        const rendered = {
            subject: this.interpolateString(template.subject, variables),
            html: this.interpolateString(template.html, variables),
            text: template.text ? this.interpolateString(template.text, variables) : null
        };

        return rendered;
    }

    interpolateString(template, variables) {
        return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
            return variables[key] || match;
        });
    }

    async processEmailQueue() {
        if (this.isProcessingQueue || this.emailQueue.length === 0) {
            return;
        }

        this.isProcessingQueue = true;

        try {
            while (this.emailQueue.length > 0) {
                const email = this.emailQueue.shift();

                // Check if scheduled for future
                if (email.scheduledFor && new Date(email.scheduledFor) > new Date()) {
                    // Put back in queue for later
                    this.emailQueue.push(email);
                    continue;
                }

                try {
                    await this.sendEmailNow(email);
                    await this.logEmailEvent('email_sent', email.id, {
                        to: email.to,
                        template: email.template
                    });

                } catch (error) {
                    console.error(`Failed to send email ${email.id}:`, error);
                    
                    // Retry logic
                    email.retryCount++;
                    if (email.retryCount < 3) {
                        // Add back to queue with delay
                        setTimeout(() => {
                            this.emailQueue.push(email);
                        }, Math.pow(2, email.retryCount) * 1000); // Exponential backoff
                        
                        await this.logEmailEvent('email_retry', email.id, {
                            retryCount: email.retryCount,
                            error: error.message
                        });
                    } else {
                        await this.logEmailEvent('email_failed', email.id, {
                            finalError: error.message,
                            retryCount: email.retryCount
                        });
                    }
                }

                // Small delay between emails to avoid rate limiting
                await new Promise(resolve => setTimeout(resolve, 100));
            }

        } finally {
            this.isProcessingQueue = false;
        }
    }

    async sendEmailNow(emailData) {
        if (!this.transporter) {
            // Mock mode - just log the email
            console.log(`MOCK EMAIL: To: ${emailData.to}, Subject: ${emailData.subject}`);
            return;
        }

        const mailOptions = {
            from: `${this.config.fromName} <${this.config.fromEmail}>`,
            to: emailData.to,
            subject: emailData.subject,
            html: emailData.html,
            text: emailData.text,
            replyTo: this.config.replyTo
        };

        const result = await this.transporter.sendMail(mailOptions);
        console.log(`Email sent to ${emailData.to}: ${result.messageId}`);
        
        return result;
    }

    startQueueProcessor() {
        // Process queue every 30 seconds
        setInterval(() => {
            if (this.emailQueue.length > 0 && !this.isProcessingQueue) {
                this.processEmailQueue();
            }
        }, 30000);
    }

    // Convenience methods for common email types
    async sendWelcomeEmail(participant) {
        return await this.sendEmail(participant.email, 'welcome', {
            firstName: participant.first_name,
            email: participant.email,
            techLevel: participant.tech_level,
            operatingSystem: participant.operating_system,
            participantId: participant.id
        });
    }

    async sendSessionReminder(participant, session) {
        const sessionDate = new Date(session.scheduled_at).toLocaleDateString();
        const sessionTime = new Date(session.scheduled_at).toLocaleTimeString();
        
        return await this.sendEmail(participant.email, 'session_reminder', {
            firstName: participant.first_name,
            sessionDate: sessionDate,
            sessionTime: sessionTime,
            estimatedDuration: session.estimated_duration,
            meetingLink: session.meeting_link || 'TBD',
            sessionId: session.id,
            operatingSystem: participant.operating_system
        }, {
            scheduledFor: new Date(new Date(session.scheduled_at).getTime() - 24 * 60 * 60 * 1000) // 24 hours before
        });
    }

    async sendSessionConfirmation(participant, session, facilitator = null) {
        const sessionDate = new Date(session.scheduled_at).toLocaleDateString();
        const sessionTime = new Date(session.scheduled_at).toLocaleTimeString();
        
        return await this.sendEmail(participant.email, 'session_scheduled', {
            firstName: participant.first_name,
            sessionDate: sessionDate,
            sessionTime: sessionTime,
            estimatedDuration: session.estimated_duration,
            meetingLink: session.meeting_link || 'TBD',
            facilitatorName: facilitator?.name || 'Luna Testing Team',
            calendarLink: this.generateCalendarLink(session)
        });
    }

    async sendThankYouEmail(participant, session, metrics = {}) {
        const sessionDate = new Date(session.scheduled_at).toLocaleDateString();
        
        return await this.sendEmail(participant.email, 'thank_you', {
            firstName: participant.first_name,
            sessionDate: sessionDate,
            actualDuration: session.duration_minutes || session.estimated_duration,
            completedTasks: metrics.completedTasks || 'N/A',
            totalTasks: metrics.totalTasks || 'N/A',
            completionScore: metrics.completionScore,
            incentiveInfo: this.getIncentiveInfo(participant)
        });
    }

    async sendFeedbackRequest(participant, session) {
        const feedbackLink = `${process.env.BASE_URL || 'http://localhost:3000'}/feedback/${session.id}`;
        
        return await this.sendEmail(participant.email, 'feedback_request', {
            firstName: participant.first_name,
            feedbackLink: feedbackLink
        }, {
            scheduledFor: new Date(Date.now() + 3 * 24 * 60 * 60 * 1000) // 3 days after session
        });
    }

    generateCalendarLink(session) {
        const startDate = new Date(session.scheduled_at);
        const endDate = new Date(startDate.getTime() + (session.estimated_duration * 60 * 1000));
        
        const formatDate = (date) => {
            return date.toISOString().replace(/[:-]/g, '').split('.')[0] + 'Z';
        };

        const title = encodeURIComponent('Luna User Testing Session');
        const details = encodeURIComponent(`Join us for a Luna testing session. Meeting link: ${session.meeting_link || 'TBD'}`);
        
        return `https://calendar.google.com/calendar/render?action=TEMPLATE&text=${title}&dates=${formatDate(startDate)}/${formatDate(endDate)}&details=${details}`;
    }

    getIncentiveInfo(participant) {
        // Customize based on your incentive program
        return "As a thank you for your participation, you'll receive a $25 Amazon gift card within 5 business days.";
    }

    async getEmailHistory(participantId, limit = 50) {
        try {
            const logs = await this.db.db.all(
                `SELECT * FROM system_logs 
                 WHERE component = 'EmailService' 
                 AND JSON_EXTRACT(data, '$.participantId') = ?
                 ORDER BY created_at DESC 
                 LIMIT ?`,
                [participantId, limit]
            );

            return logs.map(log => ({
                id: log.id,
                event: log.message,
                timestamp: log.created_at,
                data: JSON.parse(log.data || '{}')
            }));

        } catch (error) {
            console.error('Email history error:', error);
            return [];
        }
    }

    async getEmailStatistics() {
        try {
            const stats = await this.db.db.get(`
                SELECT 
                    COUNT(*) as total_emails,
                    SUM(CASE WHEN message = 'email_sent' THEN 1 ELSE 0 END) as sent_emails,
                    SUM(CASE WHEN message = 'email_failed' THEN 1 ELSE 0 END) as failed_emails,
                    SUM(CASE WHEN message = 'email_retry' THEN 1 ELSE 0 END) as retry_attempts
                FROM system_logs 
                WHERE component = 'EmailService'
                AND created_at >= datetime('now', '-30 days')
            `);

            const templateStats = await this.db.db.all(`
                SELECT 
                    JSON_EXTRACT(data, '$.template') as template_name,
                    COUNT(*) as usage_count
                FROM system_logs 
                WHERE component = 'EmailService' 
                AND message = 'email_sent'
                AND created_at >= datetime('now', '-30 days')
                GROUP BY JSON_EXTRACT(data, '$.template')
                ORDER BY usage_count DESC
            `);

            return {
                overview: stats,
                templateUsage: templateStats,
                queueSize: this.emailQueue.length,
                isProcessing: this.isProcessingQueue,
                generatedAt: new Date()
            };

        } catch (error) {
            console.error('Email statistics error:', error);
            return {
                overview: { total_emails: 0, sent_emails: 0, failed_emails: 0 },
                templateUsage: [],
                queueSize: this.emailQueue.length
            };
        }
    }

    async logEmailEvent(eventType, emailId, data) {
        try {
            await this.db.db.run(
                `INSERT INTO system_logs (level, component, message, data)
                 VALUES (?, ?, ?, ?)`,
                ['info', 'EmailService', eventType, JSON.stringify({
                    emailId,
                    ...data
                })]
            );
        } catch (error) {
            console.error('Email event logging error:', error);
            // Don't throw - logging errors shouldn't break email functionality
        }
    }

    async shutdown() {
        try {
            // Process remaining emails in queue
            if (this.emailQueue.length > 0) {
                console.log(`Processing ${this.emailQueue.length} remaining emails before shutdown...`);
                await this.processEmailQueue();
            }

            // Close transporter
            if (this.transporter) {
                this.transporter.close();
            }

            console.log('Email service shutdown complete');
        } catch (error) {
            console.error('Email service shutdown error:', error);
        }
    }
}