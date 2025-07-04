/**
 * File Service - Handles file uploads, downloads, and artifact management
 */

import fs from 'fs/promises';
import path from 'path';
import multer from 'multer';
import { v4 as uuidv4 } from 'uuid';
import crypto from 'crypto';

export class FileService {
    constructor(database, config = {}) {
        this.db = database;
        this.config = {
            uploadDir: config.uploadDir || path.join(process.cwd(), 'uploads'),
            maxFileSize: config.maxFileSize || 50 * 1024 * 1024, // 50MB
            allowedMimeTypes: config.allowedMimeTypes || [
                'image/jpeg',
                'image/png', 
                'image/gif',
                'image/webp',
                'video/mp4',
                'video/webm',
                'audio/wav',
                'audio/mp3',
                'application/pdf',
                'text/plain',
                'application/json',
                'application/zip'
            ],
            retentionDays: config.retentionDays || 90,
            ...config
        };

        this.initializeStorage();
        this.setupMulter();
    }

    async initializeStorage() {
        try {
            // Create main upload directory
            await fs.mkdir(this.config.uploadDir, { recursive: true });
            
            // Create subdirectories for different file types
            const subdirs = ['sessions', 'participants', 'screenshots', 'recordings', 'logs', 'exports'];
            for (const subdir of subdirs) {
                await fs.mkdir(path.join(this.config.uploadDir, subdir), { recursive: true });
            }

            console.log(`File storage initialized at: ${this.config.uploadDir}`);
        } catch (error) {
            console.error('File storage initialization error:', error);
            throw error;
        }
    }

    setupMulter() {
        // Configure multer for file uploads
        const storage = multer.diskStorage({
            destination: (req, file, cb) => {
                const subdir = this.getSubdirectory(file.fieldname, file.mimetype);
                const uploadPath = path.join(this.config.uploadDir, subdir);
                cb(null, uploadPath);
            },
            filename: (req, file, cb) => {
                const uniqueId = uuidv4();
                const extension = path.extname(file.originalname);
                const sanitizedName = this.sanitizeFilename(file.originalname);
                cb(null, `${uniqueId}_${sanitizedName}${extension}`);
            }
        });

        this.multerUpload = multer({
            storage: storage,
            limits: {
                fileSize: this.config.maxFileSize,
                files: 10 // Max 10 files per request
            },
            fileFilter: (req, file, cb) => {
                if (this.config.allowedMimeTypes.includes(file.mimetype)) {
                    cb(null, true);
                } else {
                    cb(new Error(`File type ${file.mimetype} not allowed`), false);
                }
            }
        });
    }

    getSubdirectory(fieldname, mimetype) {
        // Determine subdirectory based on field name or MIME type
        if (fieldname?.includes('screenshot') || mimetype?.startsWith('image/')) {
            return 'screenshots';
        } else if (fieldname?.includes('recording') || mimetype?.startsWith('video/') || mimetype?.startsWith('audio/')) {
            return 'recordings';
        } else if (fieldname?.includes('session')) {
            return 'sessions';
        } else if (fieldname?.includes('participant')) {
            return 'participants';
        } else if (fieldname?.includes('log')) {
            return 'logs';
        } else {
            return 'sessions'; // Default
        }
    }

    sanitizeFilename(filename) {
        // Remove dangerous characters and limit length
        return filename
            .replace(/[^a-zA-Z0-9.-]/g, '_')
            .substring(0, 100);
    }

    async saveFile(fileData, metadata = {}) {
        try {
            const fileId = uuidv4();
            const timestamp = new Date();

            // Determine file path
            const subdir = metadata.category || 'sessions';
            const extension = metadata.extension || path.extname(metadata.originalName || '') || '.bin';
            const filename = `${fileId}${extension}`;
            const filePath = path.join(this.config.uploadDir, subdir, filename);

            // Save file to disk
            if (Buffer.isBuffer(fileData)) {
                await fs.writeFile(filePath, fileData);
            } else if (typeof fileData === 'string') {
                await fs.writeFile(filePath, fileData, 'utf8');
            } else {
                throw new Error('Invalid file data type');
            }

            // Calculate file hash for integrity checking
            const fileBuffer = await fs.readFile(filePath);
            const hash = crypto.createHash('sha256').update(fileBuffer).digest('hex');
            const fileSize = fileBuffer.length;

            // Save metadata to database
            const fileRecord = await this.db.db.run(
                `INSERT INTO files (id, original_name, filename, file_path, file_size, mime_type, hash, 
                                   category, session_id, participant_id, metadata, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
                [
                    fileId,
                    metadata.originalName || filename,
                    filename,
                    filePath,
                    fileSize,
                    metadata.mimeType || 'application/octet-stream',
                    hash,
                    metadata.category || 'session',
                    metadata.sessionId || null,
                    metadata.participantId || null,
                    JSON.stringify(metadata),
                    timestamp
                ]
            );

            // Log file creation
            await this.logFileEvent('file_created', fileId, {
                originalName: metadata.originalName,
                fileSize: fileSize,
                category: metadata.category
            });

            return {
                id: fileId,
                filename: filename,
                path: filePath,
                size: fileSize,
                hash: hash,
                url: this.generateFileUrl(fileId)
            };

        } catch (error) {
            console.error('File save error:', error);
            throw error;
        }
    }

    async saveScreenshot(sessionId, screenshotData, metadata = {}) {
        try {
            const timestamp = new Date();
            const filename = `screenshot_${sessionId}_${timestamp.getTime()}.png`;
            
            return await this.saveFile(screenshotData, {
                originalName: filename,
                mimeType: 'image/png',
                category: 'screenshots',
                sessionId: sessionId,
                ...metadata
            });

        } catch (error) {
            console.error('Screenshot save error:', error);
            throw error;
        }
    }

    async saveSessionRecording(sessionId, recordingData, metadata = {}) {
        try {
            const timestamp = new Date();
            const extension = metadata.format === 'webm' ? '.webm' : '.mp4';
            const filename = `recording_${sessionId}_${timestamp.getTime()}${extension}`;
            
            return await this.saveFile(recordingData, {
                originalName: filename,
                mimeType: metadata.format === 'webm' ? 'video/webm' : 'video/mp4',
                category: 'recordings',
                sessionId: sessionId,
                duration: metadata.duration,
                ...metadata
            });

        } catch (error) {
            console.error('Recording save error:', error);
            throw error;
        }
    }

    async saveSessionLogs(sessionId, logData, metadata = {}) {
        try {
            const timestamp = new Date();
            const filename = `session_logs_${sessionId}_${timestamp.getTime()}.json`;
            
            const logJson = typeof logData === 'string' ? logData : JSON.stringify(logData, null, 2);
            
            return await this.saveFile(logJson, {
                originalName: filename,
                mimeType: 'application/json',
                category: 'logs',
                sessionId: sessionId,
                ...metadata
            });

        } catch (error) {
            console.error('Session logs save error:', error);
            throw error;
        }
    }

    async getFile(fileId) {
        try {
            const file = await this.db.db.get(
                'SELECT * FROM files WHERE id = ?',
                [fileId]
            );

            if (!file) {
                throw new Error('File not found');
            }

            // Check if file exists on disk
            try {
                await fs.access(file.file_path);
            } catch {
                throw new Error('File not found on disk');
            }

            return {
                ...file,
                metadata: JSON.parse(file.metadata || '{}'),
                url: this.generateFileUrl(fileId)
            };

        } catch (error) {
            console.error('Get file error:', error);
            throw error;
        }
    }

    async getFileData(fileId) {
        try {
            const file = await this.getFile(fileId);
            const data = await fs.readFile(file.file_path);
            
            return {
                data: data,
                mimeType: file.mime_type,
                filename: file.original_name,
                size: file.file_size
            };

        } catch (error) {
            console.error('Get file data error:', error);
            throw error;
        }
    }

    async listFiles(options = {}) {
        try {
            const {
                sessionId,
                participantId,
                category,
                mimeType,
                page = 1,
                limit = 50
            } = options;

            const offset = (page - 1) * limit;
            
            let query = 'SELECT * FROM files WHERE 1=1';
            const params = [];

            if (sessionId) {
                query += ' AND session_id = ?';
                params.push(sessionId);
            }
            if (participantId) {
                query += ' AND participant_id = ?';
                params.push(participantId);
            }
            if (category) {
                query += ' AND category = ?';
                params.push(category);
            }
            if (mimeType) {
                query += ' AND mime_type LIKE ?';
                params.push(`${mimeType}%`);
            }

            query += ' ORDER BY created_at DESC LIMIT ? OFFSET ?';
            params.push(limit, offset);

            const files = await this.db.db.all(query, params);

            // Add URLs and parse metadata
            const enrichedFiles = files.map(file => ({
                ...file,
                metadata: JSON.parse(file.metadata || '{}'),
                url: this.generateFileUrl(file.id)
            }));

            // Get total count
            let countQuery = 'SELECT COUNT(*) as total FROM files WHERE 1=1';
            const countParams = [];
            
            if (sessionId) {
                countQuery += ' AND session_id = ?';
                countParams.push(sessionId);
            }
            if (participantId) {
                countQuery += ' AND participant_id = ?';
                countParams.push(participantId);
            }
            if (category) {
                countQuery += ' AND category = ?';
                countParams.push(category);
            }
            if (mimeType) {
                countQuery += ' AND mime_type LIKE ?';
                countParams.push(`${mimeType}%`);
            }

            const totalResult = await this.db.db.get(countQuery, countParams);
            const total = totalResult.total;

            return {
                files: enrichedFiles,
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
            console.error('List files error:', error);
            throw error;
        }
    }

    async deleteFile(fileId) {
        try {
            const file = await this.getFile(fileId);
            
            // Delete from filesystem
            try {
                await fs.unlink(file.file_path);
            } catch (error) {
                console.warn(`Could not delete file from disk: ${error.message}`);
            }

            // Delete from database
            await this.db.db.run('DELETE FROM files WHERE id = ?', [fileId]);

            await this.logFileEvent('file_deleted', fileId, {
                originalName: file.original_name,
                category: file.category
            });

            return true;

        } catch (error) {
            console.error('Delete file error:', error);
            throw error;
        }
    }

    async getSessionFiles(sessionId) {
        try {
            const files = await this.listFiles({ sessionId, limit: 1000 });
            
            // Group files by category
            const grouped = {
                screenshots: [],
                recordings: [],
                logs: [],
                other: []
            };

            files.files.forEach(file => {
                const category = file.category || 'other';
                if (grouped[category]) {
                    grouped[category].push(file);
                } else {
                    grouped.other.push(file);
                }
            });

            return grouped;

        } catch (error) {
            console.error('Get session files error:', error);
            throw error;
        }
    }

    async createSessionArchive(sessionId) {
        try {
            const session = await this.db.getSession(sessionId);
            if (!session) {
                throw new Error('Session not found');
            }

            const sessionFiles = await this.getSessionFiles(sessionId);
            const archiveData = {
                session: session,
                files: sessionFiles,
                exportedAt: new Date()
            };

            // Create archive as JSON
            const archiveJson = JSON.stringify(archiveData, null, 2);
            
            return await this.saveFile(archiveJson, {
                originalName: `session_archive_${sessionId}.json`,
                mimeType: 'application/json',
                category: 'exports',
                sessionId: sessionId
            });

        } catch (error) {
            console.error('Create session archive error:', error);
            throw error;
        }
    }

    async exportParticipantData(participantId) {
        try {
            const participant = await this.db.getParticipant(participantId);
            if (!participant) {
                throw new Error('Participant not found');
            }

            // Get all participant sessions
            const sessions = await this.db.listSessions({ participantId });
            
            // Get all participant files
            const files = await this.listFiles({ participantId, limit: 1000 });

            // Get participant feedback
            const feedback = await this.db.db.all(
                'SELECT * FROM feedback WHERE participant_id = ?',
                [participantId]
            );

            const exportData = {
                participant: participant,
                sessions: sessions,
                files: files.files,
                feedback: feedback,
                exportedAt: new Date()
            };

            const exportJson = JSON.stringify(exportData, null, 2);
            
            return await this.saveFile(exportJson, {
                originalName: `participant_export_${participantId}.json`,
                mimeType: 'application/json',
                category: 'exports',
                participantId: participantId
            });

        } catch (error) {
            console.error('Export participant data error:', error);
            throw error;
        }
    }

    async getStorageStatistics() {
        try {
            const stats = await this.db.db.get(`
                SELECT 
                    COUNT(*) as total_files,
                    SUM(file_size) as total_size,
                    AVG(file_size) as avg_file_size
                FROM files
            `);

            const categoryStats = await this.db.db.all(`
                SELECT 
                    category,
                    COUNT(*) as file_count,
                    SUM(file_size) as total_size
                FROM files
                GROUP BY category
                ORDER BY total_size DESC
            `);

            const mimeTypeStats = await this.db.db.all(`
                SELECT 
                    mime_type,
                    COUNT(*) as file_count,
                    SUM(file_size) as total_size
                FROM files
                GROUP BY mime_type
                ORDER BY file_count DESC
                LIMIT 10
            `);

            const recentFiles = await this.db.db.all(`
                SELECT id, original_name, file_size, created_at
                FROM files
                ORDER BY created_at DESC
                LIMIT 10
            `);

            return {
                overview: {
                    ...stats,
                    total_size_mb: Math.round(stats.total_size / (1024 * 1024) * 100) / 100,
                    avg_file_size_mb: Math.round(stats.avg_file_size / (1024 * 1024) * 100) / 100
                },
                byCategory: categoryStats.map(cat => ({
                    ...cat,
                    total_size_mb: Math.round(cat.total_size / (1024 * 1024) * 100) / 100
                })),
                byMimeType: mimeTypeStats.map(type => ({
                    ...type,
                    total_size_mb: Math.round(type.total_size / (1024 * 1024) * 100) / 100
                })),
                recentFiles: recentFiles,
                generatedAt: new Date()
            };

        } catch (error) {
            console.error('Storage statistics error:', error);
            throw error;
        }
    }

    async cleanupOldFiles() {
        try {
            const cutoffDate = new Date();
            cutoffDate.setDate(cutoffDate.getDate() - this.config.retentionDays);

            // Get old files
            const oldFiles = await this.db.db.all(
                'SELECT * FROM files WHERE created_at < ?',
                [cutoffDate]
            );

            let deletedCount = 0;
            let errorCount = 0;

            for (const file of oldFiles) {
                try {
                    await this.deleteFile(file.id);
                    deletedCount++;
                } catch (error) {
                    console.error(`Failed to delete old file ${file.id}:`, error);
                    errorCount++;
                }
            }

            await this.logFileEvent('cleanup_completed', null, {
                deletedCount,
                errorCount,
                cutoffDate
            });

            return {
                deletedCount,
                errorCount,
                totalOldFiles: oldFiles.length
            };

        } catch (error) {
            console.error('Cleanup error:', error);
            throw error;
        }
    }

    async verifyFileIntegrity(fileId) {
        try {
            const file = await this.getFile(fileId);
            const fileData = await fs.readFile(file.file_path);
            const actualHash = crypto.createHash('sha256').update(fileData).digest('hex');
            
            const isValid = actualHash === file.hash;
            
            if (!isValid) {
                await this.logFileEvent('integrity_check_failed', fileId, {
                    expectedHash: file.hash,
                    actualHash: actualHash
                });
            }

            return {
                isValid,
                expectedHash: file.hash,
                actualHash: actualHash,
                fileSize: fileData.length,
                expectedSize: file.file_size
            };

        } catch (error) {
            console.error('File integrity verification error:', error);
            throw error;
        }
    }

    async batchVerifyIntegrity() {
        try {
            const allFiles = await this.listFiles({ limit: 10000 });
            const results = {
                total: allFiles.files.length,
                valid: 0,
                invalid: 0,
                errors: 0,
                invalidFiles: []
            };

            for (const file of allFiles.files) {
                try {
                    const verification = await this.verifyFileIntegrity(file.id);
                    if (verification.isValid) {
                        results.valid++;
                    } else {
                        results.invalid++;
                        results.invalidFiles.push({
                            id: file.id,
                            name: file.original_name,
                            verification
                        });
                    }
                } catch (error) {
                    results.errors++;
                    console.error(`Verification error for file ${file.id}:`, error);
                }
            }

            return results;

        } catch (error) {
            console.error('Batch integrity verification error:', error);
            throw error;
        }
    }

    generateFileUrl(fileId) {
        const baseUrl = process.env.BASE_URL || 'http://localhost:3000';
        return `${baseUrl}/api/files/${fileId}`;
    }

    getMulterMiddleware() {
        return this.multerUpload;
    }

    async logFileEvent(eventType, fileId, data) {
        try {
            await this.db.db.run(
                `INSERT INTO system_logs (level, component, message, data)
                 VALUES (?, ?, ?, ?)`,
                ['info', 'FileService', eventType, JSON.stringify({
                    fileId,
                    ...data
                })]
            );
        } catch (error) {
            console.error('File event logging error:', error);
            // Don't throw - logging errors shouldn't break file functionality
        }
    }
}