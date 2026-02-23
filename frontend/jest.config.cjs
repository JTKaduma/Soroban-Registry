/** Jest configuration for frontend tests */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'jsdom',
  testMatch: ['**/__tests__/**/*.test.ts', '**/?(*.)+(spec|test).ts'],
  setupFilesAfterEnv: ['<rootDir>/setupTests.ts'],
  collectCoverageFrom: [
    'lib/**/*.ts',
    '!lib/**/mock-data.ts',
  ],
  coverageDirectory: '<rootDir>/coverage',
};
