// Enable fetch mocking
import fetchMock from 'jest-fetch-mock';
fetchMock.enableMocks();

// Optional: silence console.error in tests unless explicitly checking for it
const originalError = console.error;
beforeEach(() => {
  fetchMock.resetMocks();
});

afterAll(() => {
  console.error = originalError;
});
