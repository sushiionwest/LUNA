/**
 * Participant Service - Manages user registration, screening, and lifecycle
 */

import { v4 as uuidv4 } from 'uuid';
import bcrypt from 'bcrypt';

export class ParticipantService {
    constructor(database) {
        this.db = database;
    }

    async register(data) {
        try {
            // Validate required fields
            this.validateRegistrationData(data);

            // Check if email already exists
            const existing = await this.db.getParticipantByEmail(data.email);
            if (existing) {
                throw new Error('Email already registered');
            }

            // Determine user segment based on tech level
            const segment = this.determineSegment(data.techLevel);

            // Create participant record
            const participant = await this.db.createParticipant({
                email: data.email,
                firstName: data.firstName,
                lastName: data.lastName || '',
                phone: data.phone || '',
                operatingSystem: data.operatingSystem,
                techLevel: data.techLevel,
                useCaseInterest: data.useCaseInterest || '',
                segment: segment,
                notes: data.notes || ''
            });

            // Log registration event
            await this.logEvent('participant_registered', participant.id, {
                segment: segment,
                techLevel: data.techLevel,
                source: data.source || 'direct'
            });

            return participant;

        } catch (error) {
            console.error('Registration error:', error);
            throw error;
        }
    }

    validateRegistrationData(data) {
        const required = ['email', 'firstName', 'operatingSystem', 'techLevel'];
        const missing = required.filter(field => !data[field]);
        
        if (missing.length > 0) {
            throw new Error(`Missing required fields: ${missing.join(', ')}`);
        }

        // Validate email format
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        if (!emailRegex.test(data.email)) {
            throw new Error('Invalid email format');
        }

        // Validate tech level
        const validTechLevels = ['non-technical', 'basic', 'intermediate', 'advanced'];
        if (!validTechLevels.includes(data.techLevel)) {
            throw new Error('Invalid tech level');
        }

        // Validate operating system
        const validOS = ['Windows', 'macOS', 'Linux'];
        if (!validOS.includes(data.operatingSystem)) {
            throw new Error('Invalid operating system');
        }
    }

    determineSegment(techLevel) {
        const segmentMap = {
            'non-technical': 'non-technical',
            'basic': 'non-technical',
            'intermediate': 'semi-technical',
            'advanced': 'technical'
        };
        return segmentMap[techLevel] || 'non-technical';
    }

    async getById(id) {
        try {
            const participant = await this.db.getParticipant(id);
            if (!participant) {
                throw new Error('Participant not found');
            }

            // Get additional data
            const sessions = await this.db.listSessions({ participantId: id });
            const feedback = await this.db.db.all(
                'SELECT * FROM feedback WHERE participant_id = ? ORDER BY submitted_at DESC',
                [id]
            );

            return {
                ...participant,
                sessions: sessions,
                feedback: feedback,
                statistics: await this.getParticipantStatistics(id)
            };

        } catch (error) {
            console.error('Error getting participant:', error);
            throw error;
        }
    }

    async update(id, data) {
        try {
            // Validate the participant exists
            const existing = await this.db.getParticipant(id);
            if (!existing) {
                throw new Error('Participant not found');
            }

            // Update segment if tech level changed
            if (data.techLevel && data.techLevel !== existing.tech_level) {
                data.segment = this.determineSegment(data.techLevel);
            }

            const updated = await this.db.updateParticipant(id, data);

            // Log update event
            await this.logEvent('participant_updated', id, {
                changes: Object.keys(data),
                previousSegment: existing.segment,
                newSegment: data.segment || existing.segment
            });

            return updated;

        } catch (error) {
            console.error('Error updating participant:', error);
            throw error;
        }
    }

    async updateStatus(id, status, notes) {
        try {
            const validStatuses = ['pending', 'screened', 'scheduled', 'active', 'completed', 'dropped', 'ineligible'];
            if (!validStatuses.includes(status)) {
                throw new Error('Invalid status');
            }

            const participant = await this.db.updateParticipant(id, { 
                status: status,
                notes: notes || ''
            });

            // Log status change
            await this.logEvent('status_changed', id, {
                newStatus: status,
                notes: notes
            });

            return participant;

        } catch (error) {
            console.error('Error updating status:', error);
            throw error;
        }
    }

    async list(options = {}) {
        try {
            const {
                page = 1,
                limit = 50,
                segment,
                status,
                techLevel,
                operatingSystem
            } = options;

            const offset = (page - 1) * limit;

            const filters = {
                segment,
                status,
                techLevel,
                operatingSystem,
                limit,
                offset
            };

            // Remove undefined filters
            Object.keys(filters).forEach(key => {
                if (filters[key] === undefined) {
                    delete filters[key];
                }
            });

            const participants = await this.db.listParticipants(filters);

            // Get total count for pagination
            let countQuery = 'SELECT COUNT(*) as total FROM participants WHERE 1=1';
            const countParams = [];

            if (segment) {
                countQuery += ' AND segment = ?';
                countParams.push(segment);
            }
            if (status) {
                countQuery += ' AND status = ?';
                countParams.push(status);
            }
            if (techLevel) {
                countQuery += ' AND tech_level = ?';
                countParams.push(techLevel);
            }
            if (operatingSystem) {
                countQuery += ' AND operating_system = ?';
                countParams.push(operatingSystem);
            }

            const totalResult = await this.db.db.get(countQuery, countParams);
            const total = totalResult.total;

            return {
                participants,
                pagination: {
                    page,
                    limit,
                    total,
                    totalPages: Math.ceil(total / limit),
                    hasNext: page * limit < total,
                    hasPrevious: page > 1
                }
            };

        } catch (error) {
            console.error('Error listing participants:', error);
            throw error;
        }
    }

    async getSegmentDistribution() {
        try {
            const distribution = await this.db.db.all(`
                SELECT 
                    segment,
                    COUNT(*) as count,
                    ROUND(COUNT(*) * 100.0 / (SELECT COUNT(*) FROM participants), 2) as percentage
                FROM participants 
                GROUP BY segment
                ORDER BY count DESC
            `);

            return distribution;

        } catch (error) {
            console.error('Error getting segment distribution:', error);
            throw error;
        }
    }

    async getRecruitmentStats() {
        try {
            const stats = await this.db.db.get(`
                SELECT 
                    COUNT(*) as total_registered,
                    SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                    SUM(CASE WHEN status = 'screened' THEN 1 ELSE 0 END) as screened,
                    SUM(CASE WHEN status = 'scheduled' THEN 1 ELSE 0 END) as scheduled,
                    SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) as active,
                    SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
                    SUM(CASE WHEN status = 'dropped' THEN 1 ELSE 0 END) as dropped,
                    SUM(CASE WHEN status = 'ineligible' THEN 1 ELSE 0 END) as ineligible
                FROM participants
            `);

            const registrationsByDate = await this.db.db.all(`
                SELECT 
                    DATE(created_at) as date,
                    COUNT(*) as registrations
                FROM participants 
                WHERE created_at >= datetime('now', '-30 days')
                GROUP BY DATE(created_at)
                ORDER BY date DESC
            `);

            return {
                overview: stats,
                dailyRegistrations: registrationsByDate,
                conversionRate: stats.completed / stats.total_registered * 100
            };

        } catch (error) {
            console.error('Error getting recruitment stats:', error);
            throw error;
        }
    }

    async screenParticipant(id, screeningData) {
        try {
            const participant = await this.db.getParticipant(id);
            if (!participant) {
                throw new Error('Participant not found');
            }

            // Calculate screening score
            const score = this.calculateScreeningScore(participant, screeningData);
            
            // Determine eligibility
            const isEligible = score >= 70; // 70% threshold
            const newStatus = isEligible ? 'screened' : 'ineligible';

            // Update participant with screening results
            await this.db.updateParticipant(id, {
                status: newStatus,
                notes: `Screening score: ${score}%. ${screeningData.notes || ''}`
            });

            // Log screening event
            await this.logEvent('participant_screened', id, {
                score: score,
                eligible: isEligible,
                criteria: screeningData
            });

            return {
                score,
                eligible: isEligible,
                status: newStatus,
                recommendations: this.getScreeningRecommendations(score, screeningData)
            };

        } catch (error) {
            console.error('Error screening participant:', error);
            throw error;
        }
    }

    calculateScreeningScore(participant, screening) {
        let score = 0;
        let maxScore = 0;

        // Tech level appropriateness (25 points)
        maxScore += 25;
        const techLevelScores = {
            'non-technical': 25,
            'basic': 20,
            'intermediate': 23,
            'advanced': 22
        };
        score += techLevelScores[participant.tech_level] || 0;

        // Availability (20 points)
        maxScore += 20;
        if (screening.available) score += 20;
        else if (screening.partiallyAvailable) score += 10;

        // Computer access (20 points)
        maxScore += 20;
        if (screening.hasAdminRights) score += 10;
        if (screening.hasReliableInternet) score += 10;

        // Motivation and interest (15 points)
        maxScore += 15;
        if (screening.motivationLevel === 'high') score += 15;
        else if (screening.motivationLevel === 'medium') score += 10;
        else if (screening.motivationLevel === 'low') score += 5;

        // Communication skills (10 points)
        maxScore += 10;
        if (screening.communicationClear) score += 10;

        // Segment balance (10 points)
        maxScore += 10;
        // Add bonus points for underrepresented segments
        // This would require checking current segment distribution
        score += 5; // Default bonus

        return Math.round((score / maxScore) * 100);
    }

    getScreeningRecommendations(score, screening) {
        const recommendations = [];

        if (score >= 90) {
            recommendations.push('Excellent candidate - prioritize for scheduling');
        } else if (score >= 80) {
            recommendations.push('Good candidate - schedule when available');
        } else if (score >= 70) {
            recommendations.push('Acceptable candidate - schedule if needed');
        } else {
            recommendations.push('Not recommended for current study');
        }

        if (!screening.hasAdminRights) {
            recommendations.push('May need technical assistance with installation');
        }

        if (screening.motivationLevel === 'low') {
            recommendations.push('Consider additional incentives or shorter session');
        }

        return recommendations;
    }

    async getParticipantStatistics(id) {
        try {
            const sessionStats = await this.db.db.get(`
                SELECT 
                    COUNT(*) as total_sessions,
                    SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed_sessions,
                    AVG(duration_minutes) as avg_duration
                FROM sessions 
                WHERE participant_id = ?
            `, [id]);

            const eventStats = await this.db.db.get(`
                SELECT COUNT(*) as total_events
                FROM session_events se
                JOIN sessions s ON se.session_id = s.id
                WHERE s.participant_id = ?
            `, [id]);

            const feedbackStats = await this.db.db.get(`
                SELECT 
                    COUNT(*) as feedback_count,
                    AVG(nps_score) as avg_nps,
                    AVG(install_rating) as avg_install_rating
                FROM feedback 
                WHERE participant_id = ?
            `, [id]);

            return {
                sessions: sessionStats,
                events: eventStats,
                feedback: feedbackStats
            };

        } catch (error) {
            console.error('Error getting participant statistics:', error);
            return {};
        }
    }

    async logEvent(eventType, participantId, data) {
        try {
            await this.db.db.run(
                `INSERT INTO system_logs (level, component, message, data)
                 VALUES (?, ?, ?, ?)`,
                ['info', 'ParticipantService', eventType, JSON.stringify({
                    participantId,
                    ...data
                })]
            );
        } catch (error) {
            console.error('Error logging event:', error);
            // Don't throw - logging errors shouldn't break main functionality
        }
    }

    async getUpcomingSessions(participantId) {
        try {
            return await this.db.db.all(`
                SELECT * FROM sessions 
                WHERE participant_id = ? 
                AND status IN ('scheduled', 'confirmed')
                AND scheduled_at > datetime('now')
                ORDER BY scheduled_at ASC
            `, [participantId]);

        } catch (error) {
            console.error('Error getting upcoming sessions:', error);
            throw error;
        }
    }

    async sendReminder(participantId, reminderType = 'session') {
        try {
            const participant = await this.db.getParticipant(participantId);
            if (!participant) {
                throw new Error('Participant not found');
            }

            // Log reminder sent
            await this.logEvent('reminder_sent', participantId, {
                type: reminderType,
                email: participant.email
            });

            return true;

        } catch (error) {
            console.error('Error sending reminder:', error);
            throw error;
        }
    }

    async markNoShow(participantId, sessionId) {
        try {
            // Update session status
            await this.db.updateSession(sessionId, {
                status: 'no-show'
            });

            // Update participant status if multiple no-shows
            const noShowCount = await this.db.db.get(`
                SELECT COUNT(*) as count 
                FROM sessions 
                WHERE participant_id = ? AND status = 'no-show'
            `, [participantId]);

            if (noShowCount.count >= 2) {
                await this.updateStatus(participantId, 'dropped', 'Multiple no-shows');
            }

            await this.logEvent('no_show_recorded', participantId, {
                sessionId,
                totalNoShows: noShowCount.count
            });

            return true;

        } catch (error) {
            console.error('Error marking no-show:', error);
            throw error;
        }
    }

    async generateParticipantReport(id) {
        try {
            const participant = await this.getById(id);
            const stats = await this.getParticipantStatistics(id);

            return {
                participant,
                statistics: stats,
                generatedAt: new Date().toISOString(),
                summary: {
                    completionRate: (stats.sessions.completed_sessions / stats.sessions.total_sessions) * 100,
                    averageEngagement: stats.events.total_events / stats.sessions.total_sessions,
                    satisfactionScore: stats.feedback.avg_nps || 0
                }
            };

        } catch (error) {
            console.error('Error generating participant report:', error);
            throw error;
        }
    }
}