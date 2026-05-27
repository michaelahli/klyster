/**
 * Thin HTTP client for the Klyster web API.
 *
 * Handles JSON decoding, sensible error messages, and a single retry on
 * transient network failures. Real resilience (circuit breaker, backoff)
 * lives in the Rust `analytics` layer; the UI just needs enough to keep
 * the dashboard usable across brief blips.
 */

const DEFAULT_TIMEOUT_MS = 10_000;

export class ApiError extends Error {
  readonly status: number;
  readonly url: string;

  constructor(message: string, status: number, url: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.url = url;
  }
}

export interface FetchOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
  retry?: boolean;
}

/**
 * Fetch JSON from the API and parse the body.
 *
 * Throws `ApiError` for non-2xx responses; the caller can inspect
 * `error.status` to react to specific codes.
 */
export async function getJson<T>(path: string, options: FetchOptions = {}): Promise<T> {
  const { timeoutMs = DEFAULT_TIMEOUT_MS, retry = true } = options;
  const url = path.startsWith('/') ? path : `/${path}`;

  const attempt = async (): Promise<T> => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    if (options.signal) {
      options.signal.addEventListener('abort', () => controller.abort(), {
        once: true,
      });
    }

    try {
      const response = await fetch(url, {
        signal: controller.signal,
        headers: { Accept: 'application/json' },
      });
      if (!response.ok) {
        let message = `${response.status} ${response.statusText}`;
        try {
          const body = (await response.json()) as { error?: { message?: string } };
          if (body?.error?.message) {
            message = body.error.message;
          }
        } catch {
          // Non-JSON error body. Keep status/statusText.
        }
        throw new ApiError(message, response.status, url);
      }
      return (await response.json()) as T;
    } finally {
      clearTimeout(timer);
    }
  };

  try {
    return await attempt();
  } catch (err) {
    if (
      retry &&
      !(err instanceof ApiError) &&
      !(err instanceof DOMException && err.name === 'AbortError')
    ) {
      // Single retry for network/timeout errors. ApiError responses are not
      // retried — the server already answered and a retry won't change that.
      return attempt();
    }
    throw err;
  }
}
