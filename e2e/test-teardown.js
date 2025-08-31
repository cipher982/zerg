/**
 * Global test teardown - runs once after all tests complete
 * Modern testing practices 2025: Automatic cleanup of test artifacts
 */

async function globalTeardown(config) {
  console.log('🧹 Starting test environment cleanup...');
  
  const { spawn } = require('child_process');
  const path = require('path');
  
  try {
    // Call Python cleanup script to remove all test databases
    const cleanup = spawn('python3', ['-c', `
import sys
sys.path.insert(0, '${path.resolve('../backend')}')
from zerg.test_db_manager import cleanup_test_databases
cleanup_test_databases()
print("✅ Test database cleanup completed")
    `], {
      cwd: path.resolve('../backend'),
      stdio: 'inherit'
    });
    cleanup.on('error', (err) => {
      throw new Error(`Python3 is required for E2E cleanup. Set PYTHON to a valid python3 path or install python3. Original error: ${err.message}`);
    });
    
    await new Promise((resolve, reject) => {
      cleanup.on('close', (code) => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(`Cleanup failed with code ${code}`));
        }
      });
    });
    
    console.log('✅ Test environment cleanup completed');
    
  } catch (error) {
    console.error('❌ Test cleanup failed:', error.message);
    // Continue anyway - don't fail the entire test run due to cleanup issues
  }
}

module.exports = globalTeardown;
