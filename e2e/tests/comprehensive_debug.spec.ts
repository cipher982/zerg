import { test, expect } from './fixtures';

/**
 * COMPREHENSIVE DEBUG TEST
 * 
 * This test systematically checks every aspect of the database isolation:
 * 1. Middleware activation
 * 2. Header processing
 * 3. Database creation
 * 4. Table initialization
 * 5. CRUD operations
 */

test.describe('Comprehensive Debug', () => {
  test('Complete system debug and diagnosis', async ({ page }, testInfo) => {
    console.log('🔍 Starting comprehensive debug test...');
    
    // Get the worker ID from test info
    const workerId = String(testInfo.workerIndex);
    console.log('📊 Worker ID:', workerId);
    console.log('📊 NODE_ENV:', process.env.NODE_ENV);
    
    // Test 1: Basic connectivity
    console.log('🔍 Test 1: Basic connectivity');
    try {
      const response = await page.request.get('http://localhost:8001/');
      console.log('📊 Basic connectivity status:', response.status());
      if (response.ok()) {
        console.log('✅ Backend is accessible');
      } else {
        console.log('❌ Backend connectivity failed');
        return;
      }
    } catch (error) {
      console.log('❌ Backend connectivity error:', error);
      return;
    }
    
    // Test 2: Header transmission
    console.log('🔍 Test 2: Header transmission');
    try {
      const response = await page.request.get('http://localhost:8001/', {
        headers: {
          'X-Debug-Test': 'header-test'
        }
      });
      console.log('📊 Header transmission status:', response.status());
      console.log('✅ Headers can be sent');
    } catch (error) {
      console.log('❌ Header transmission error:', error);
    }
    
    // Test 3: Agent endpoint - GET (should work)
    console.log('🔍 Test 3: Agent GET endpoint');
    try {
      const response = await page.request.get('http://localhost:8001/api/agents');
      console.log('📊 Agent GET status:', response.status());
      if (response.ok()) {
        const agents = await response.json();
        console.log('📊 Agent GET count:', agents.length);
        console.log('✅ Agent GET endpoint working');
      } else {
        const error = await response.text();
        console.log('❌ Agent GET failed:', error.substring(0, 200));
      }
    } catch (error) {
      console.log('❌ Agent GET error:', error);
    }
    
    // Test 4: Different HTTP methods to test database
    console.log('🔍 Test 4: Testing different database operations');
    
    // Try a simpler POST endpoint first
    try {
      console.log('📊 Testing user endpoint...');
      const userResponse = await page.request.get('http://localhost:8001/api/users/me');
      console.log('📊 User endpoint status:', userResponse.status());
      if (userResponse.ok()) {
        const user = await userResponse.json();
        console.log('📊 User data available:', !!user);
      }
    } catch (error) {
      console.log('📊 User endpoint error:', error);
    }
    
    // Test 5: Try creating a workflow instead of agent
    console.log('🔍 Test 5: Workflow creation test');
    try {
      const workflowResponse = await page.request.post('http://localhost:8001/api/workflows', {
        headers: {
          'Content-Type': 'application/json',
        },
        data: {
          name: `Test Workflow ${workerId}`,
          description: 'Test workflow for debugging',
        }
      });
      console.log('📊 Workflow creation status:', workflowResponse.status());
      if (workflowResponse.ok()) {
        const workflow = await workflowResponse.json();
        console.log('📊 Workflow created ID:', workflow.id);
        console.log('✅ Workflow creation working');
      } else {
        const error = await workflowResponse.text();
        console.log('❌ Workflow creation failed:', error.substring(0, 200));
      }
    } catch (error) {
      console.log('❌ Workflow creation error:', error);
    }
    
    // Test 6: Agent creation with minimal data
    console.log('🔍 Test 6: Minimal agent creation');
    try {
      const agentResponse = await page.request.post('http://localhost:8001/api/agents', {
        headers: {
          'Content-Type': 'application/json',
        },
        data: {
          name: 'Minimal Agent',
          system_instructions: 'System instructions',
          task_instructions: 'Task instructions',
          model: 'gpt-mock', // Use mock model to avoid external dependencies
        }
      });
      console.log('📊 Minimal agent creation status:', agentResponse.status());
      if (agentResponse.ok()) {
        const agent = await agentResponse.json();
        console.log('📊 Minimal agent created ID:', agent.id);
        console.log('✅ Agent creation working with mock model');
      } else {
        const error = await agentResponse.text();
        console.log('❌ Minimal agent creation failed:', error.substring(0, 400));
        
        // If it's a 422, try to parse the validation error
        if (agentResponse.status() === 422) {
          try {
            const errorJson = JSON.parse(error);
            console.log('📊 Validation errors:', JSON.stringify(errorJson, null, 2));
          } catch (e) {
            console.log('📊 Could not parse validation error');
          }
        }
      }
    } catch (error) {
      console.log('❌ Minimal agent creation error:', error);
    }
    
    // Test 7: Database introspection
    console.log('🔍 Test 7: Database introspection');
    try {
      // Try to access an admin endpoint that might give us database info
      const adminResponse = await page.request.get('http://localhost:8001/api/system/health');
      console.log('📊 System health status:', adminResponse.status());
      if (adminResponse.ok()) {
        const health = await adminResponse.text();
        console.log('📊 System health:', health.substring(0, 200));
      }
    } catch (error) {
      console.log('📊 System health error:', error);
    }
    
    console.log('✅ Comprehensive debug test complete');
  });
});