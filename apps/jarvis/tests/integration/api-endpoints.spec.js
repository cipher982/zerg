import { test, expect } from '@playwright/test';

test.describe('API Server Endpoints', () => {
  test('should return valid session token from /session endpoint', async ({ request }) => {
    const response = await request.get('http://localhost:8787/session');
    expect(response.status()).toBe(200);
    
    const data = await response.json();
    
    // Validate response structure
    expect(data.value).toMatch(/^ek_[a-f0-9]+$/);
    expect(data.expires_at).toBeGreaterThan(Date.now() / 1000);
    expect(data.session).toBeDefined();
    expect(data.session.type).toBe('realtime');
    expect(data.session.model).toBe('gpt-realtime');
    expect(data.session.audio.output.voice).toBe('verse');
  });

  test('should handle /tool endpoint with WHOOP mock', async ({ request }) => {
    const response = await request.post('http://localhost:8787/tool', {
      data: {
        name: 'whoop.get_daily',
        args: { date: '2025-01-01' }
      }
    });
    
    expect(response.status()).toBe(200);
    const data = await response.json();
    
    expect(data.date).toBe('2025-01-01');
    expect(data.recovery_score).toBe(62);
    expect(data.sleep).toBeDefined();
    expect(data.sleep.duration).toBe(7.1);
  });

  test('should handle /tool endpoint with unknown tool', async ({ request }) => {
    const response = await request.post('http://localhost:8787/tool', {
      data: {
        name: 'unknown.tool',
        args: { test: 'data' }
      }
    });
    
    expect(response.status()).toBe(200);
    const data = await response.json();
    
    expect(data.ok).toBe(true);
    expect(data.echo.name).toBe('unknown.tool');
    expect(data.echo.args.test).toBe('data');
  });

  test('should handle /tool endpoint with no body', async ({ request }) => {
    const response = await request.post('http://localhost:8787/tool');
    expect(response.status()).toBe(200);
    
    const data = await response.json();
    expect(data.ok).toBe(true);
    expect(data.echo.name).toBeUndefined();
  });

  test('should handle CORS requests', async ({ request }) => {
    const response = await request.get('http://localhost:8787/session', {
      headers: {
        'Origin': 'http://localhost:8080',
        'Access-Control-Request-Method': 'GET'
      }
    });
    
    expect(response.status()).toBe(200);
    // CORS headers should be present due to app.use(cors())
  });

  test('should handle server health check', async ({ request }) => {
    // Test that server is responsive
    const response = await request.get('http://localhost:8787/session');
    expect(response.status()).toBe(200);
    
    // Response should be fast (< 2 seconds for token generation)
    const startTime = Date.now();
    await request.get('http://localhost:8787/session');
    const duration = Date.now() - startTime;
    expect(duration).toBeLessThan(2000);
  });

  test('should handle multiple concurrent session requests', async ({ request }) => {
    // Test that server can handle concurrent requests
    const promises = Array.from({ length: 5 }, () => 
      request.get('http://localhost:8787/session')
    );
    
    const responses = await Promise.all(promises);
    
    // All should succeed
    responses.forEach(response => {
      expect(response.status()).toBe(200);
    });
    
    // All should have unique tokens
    const tokens = await Promise.all(
      responses.map(r => r.json().then(data => data.value))
    );
    
    const uniqueTokens = new Set(tokens);
    expect(uniqueTokens.size).toBe(5);
  });

  test('should validate session token format', async ({ request }) => {
    const response = await request.get('http://localhost:8787/session');
    const data = await response.json();
    
    // Token should be ephemeral key format
    expect(data.value).toMatch(/^ek_[a-f0-9]{32}$/);
    
    // Session should have required Realtime API fields
    expect(data.session.object).toBe('realtime.session');
    expect(data.session.id).toMatch(/^sess_/);
    expect(data.session.audio.input.format.type).toBe('audio/pcm');
    expect(data.session.audio.input.format.rate).toBe(24000);
  });
});