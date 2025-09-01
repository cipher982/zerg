/**
 * Global test setup - runs once before all tests
 * Modern testing practices 2025: Proper test environment initialization
 */

async function globalSetup(config) {
  console.log('🚀 Setting up test environment...');
  
  // Set environment variables for test isolation
  process.env.NODE_ENV = 'test';
  process.env.TESTING = '1';
  
  // Clean up any leftover database files from previous runs
  const { spawn } = require('child_process');
  const path = require('path');
  
  try {
    // Resolve a Python interpreter ('python' or 'python3')
    const pythonCmd = (() => {
      try { require('child_process').execSync('python --version', { stdio: 'ignore' }); return 'python'; } catch {}
      try { require('child_process').execSync('python3 --version', { stdio: 'ignore' }); return 'python3'; } catch {}
      return null;
    })();

    if (!pythonCmd) {
      throw new Error("No Python interpreter found (python/python3)");
    }

    // Call Python cleanup script
    const cleanup = spawn(pythonCmd, ['-c', `
import sys
sys.path.insert(0, '${path.resolve('../backend')}')
from zerg.test_db_manager import cleanup_test_databases
cleanup_test_databases()
print("✅ Pre-test cleanup completed")
    `], {
      cwd: path.resolve('../backend'),
      stdio: 'inherit'
    });
    // If this fails, the environment must expose 'python' on PATH.

    await new Promise((resolve, reject) => {
      cleanup.on('close', (code) => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(`Cleanup failed with code ${code}`));
        }
      });
    });
  } catch (error) {
    console.warn('⚠️  Pre-test cleanup failed (this is ok for first run):', error.message);
  }
  
  console.log('✅ Test environment setup completed');
}

module.exports = globalSetup;
