/**
 * TTS integration client — talks to services/tts's hardened endpoints.
 *
 * services/tts has had several recent security/reliability passes (client
 * reuse, cache correctness, error handling, trust-proxy, security headers —
 * see commits 92a3973 / b8e86ac). This client integrates with those
 * endpoints as-is: it never disables a header or auth check to work around
 * a client-side quirk (#1418).
 *
 * Endpoints used:
 *   POST /tts/enqueue        — queue a job, returns { jobId, status }
 *   GET  /tts/job/:id        — poll job status
 *   GET  /tts/job/:id/audio  — download audio once status === "done"
 *   GET  /tts/voices         — list available voices
 *
 * TTS jobs are processed asynchronously — a caller MUST poll /tts/job/:id
 * rather than assume /tts/enqueue's response means audio is ready.
 */

import { getEnvConfig } from '../env';
import { ApiError } from './public-client';
import { fillPath } from './paths';

const config = getEnvConfig();
const BASE_URL = (config.NEXT_PUBLIC_TTS_API_URL ?? '').replace(/\/$/, '');

export type TTSJobStatus = 'pending' | 'processing' | 'done' | 'error';

export interface TTSJob {
  id: string;
  text: string;
  status: TTSJobStatus;
  outputPath?: string;
  error?: string;
  createdAt: string;
  updatedAt: string;
}

export interface EnqueueResponse {
  jobId: string;
  status: TTSJobStatus;
}

export type TTSVoices = Record<string, { voiceId: string; language: string; label: string }>;

function requireBaseUrl(): string {
  if (!BASE_URL) {
    throw new ApiError(
      'TTS is not configured (NEXT_PUBLIC_TTS_API_URL is unset).',
      0,
      'TTS_NOT_CONFIGURED',
    );
  }
  return BASE_URL;
}

/** Builds the Authorization header for a request, if a credential is supplied. */
function authHeaders(credential?: string): HeadersInit {
  return credential ? { Authorization: `Bearer ${credential}` } : {};
}

async function parseErrorResponse(res: Response): Promise<ApiError> {
  let body: unknown;
  try {
    body = await res.json();
  } catch {
    body = {};
  }
  const obj = (typeof body === 'object' && body !== null) ? body as Record<string, unknown> : {};
  const message = (obj['error'] as string | undefined) ?? res.statusText ?? `HTTP ${res.status}`;
  return new ApiError(message, res.status, res.status === 429 ? 'RATE_LIMITED' : 'TTS_ERROR');
}

/** Enqueue a TTS job. Returns immediately with a job ID — poll `getJob` for completion. */
export async function enqueue(
  text: string,
  voiceId: string,
  options?: { provider?: 'elevenlabs' | 'google'; credential?: string; bypassCache?: boolean; signal?: AbortSignal },
): Promise<EnqueueResponse> {
  const res = await fetch(`${requireBaseUrl()}/tts/enqueue`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(options?.credential),
      ...(options?.bypassCache ? { 'Cache-Control': 'no-cache' } : {}),
    },
    body: JSON.stringify({ text, voiceId, provider: options?.provider }),
    signal: options?.signal,
  });
  if (!res.ok) throw await parseErrorResponse(res);
  return res.json();
}

/** Fetch the current status of a TTS job. */
export async function getJob(jobId: string, credential?: string, signal?: AbortSignal): Promise<TTSJob> {
  const res = await fetch(`${requireBaseUrl()}${fillPath('/tts/job/{id}', 'id', jobId)}`, {
    headers: authHeaders(credential),
    signal,
  });
  if (!res.ok) throw await parseErrorResponse(res);
  return res.json();
}

/** Download the generated audio for a completed job as a Blob. Throws if the job isn't done yet. */
export async function getJobAudio(jobId: string, credential?: string, signal?: AbortSignal): Promise<Blob> {
  const res = await fetch(`${requireBaseUrl()}${fillPath('/tts/job/{id}/audio', 'id', jobId)}`, {
    headers: authHeaders(credential),
    signal,
  });
  if (!res.ok) throw await parseErrorResponse(res);
  return res.blob();
}

/** List available TTS voices. */
export async function getVoices(credential?: string, signal?: AbortSignal): Promise<TTSVoices> {
  const res = await fetch(`${requireBaseUrl()}/tts/voices`, {
    headers: authHeaders(credential),
    signal,
  });
  if (!res.ok) throw await parseErrorResponse(res);
  return res.json();
}

export interface PollOptions {
  credential?: string;
  /** Delay between polls, in ms. Defaults to 1000. */
  intervalMs?: number;
  /** Give up and reject after this long, in ms. Defaults to 60000. */
  timeoutMs?: number;
  signal?: AbortSignal;
}

/**
 * Poll `/tts/job/:id` until the job reaches a terminal state ("done" or
 * "error"), rather than assuming the job is ready right after enqueueing.
 * Rejects with ApiError('TTS_TIMEOUT') if `timeoutMs` elapses first.
 */
export async function pollJob(jobId: string, options: PollOptions = {}): Promise<TTSJob> {
  const intervalMs = options.intervalMs ?? 1000;
  const timeoutMs = options.timeoutMs ?? 60_000;
  const deadline = Date.now() + timeoutMs;

  for (;;) {
    const job = await getJob(jobId, options.credential, options.signal);
    if (job.status === 'done' || job.status === 'error') return job;

    if (Date.now() >= deadline) {
      throw new ApiError(`TTS job ${jobId} did not complete within ${timeoutMs}ms.`, 0, 'TTS_TIMEOUT');
    }

    await new Promise<void>((resolve, reject) => {
      const timer = setTimeout(resolve, intervalMs);
      options.signal?.addEventListener('abort', () => {
        clearTimeout(timer);
        reject(new DOMException('Aborted', 'AbortError'));
      }, { once: true });
    });
  }
}

/** Enqueue a job, poll it to completion, and return its audio as a Blob. */
export async function synthesize(
  text: string,
  voiceId: string,
  options?: { provider?: 'elevenlabs' | 'google'; credential?: string; bypassCache?: boolean; signal?: AbortSignal; pollIntervalMs?: number; timeoutMs?: number },
): Promise<Blob> {
  const { jobId } = await enqueue(text, voiceId, options);
  const job = await pollJob(jobId, {
    credential: options?.credential,
    intervalMs: options?.pollIntervalMs,
    timeoutMs: options?.timeoutMs,
    signal: options?.signal,
  });
  if (job.status === 'error') {
    throw new ApiError(job.error ?? 'TTS job failed.', 0, 'TTS_JOB_FAILED');
  }
  return getJobAudio(jobId, options?.credential, options?.signal);
}
