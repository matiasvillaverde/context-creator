/**
 * API Tests
 * Testing the REST API endpoints
 */

const request = require('supertest');
const app = require('../src/index');

describe('API Endpoints', () => {
    describe('GET /', () => {
        it('should return welcome message', async () => {
            const response = await request(app).get('/');
            expect(response.status).toBe(200);
            expect(response.body).toHaveProperty('message');
            expect(response.body.message).toBe('Welcome to Sample Mixed Project');
        });
    });

    describe('GET /api/users', () => {
        it('should return list of users', async () => {
            const response = await request(app).get('/api/users');
            expect(response.status).toBe(200);
            expect(response.body).toHaveProperty('users');
            expect(Array.isArray(response.body.users)).toBe(true);
            expect(response.body.users.length).toBeGreaterThan(0);
        });

        it('should return user count', async () => {
            const response = await request(app).get('/api/users');
            expect(response.body).toHaveProperty('count');
            expect(response.body.count).toBe(response.body.users.length);
        });
    });

    describe('Error Handling', () => {
        it('should return 404 for unknown routes', async () => {
            const response = await request(app).get('/unknown-route');
            expect(response.status).toBe(404);
        });
    });
});