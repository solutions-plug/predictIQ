"use client";

import { useEffect, useState } from "react";
import { resolveApiOrigin } from "@/lib/env";

const STORAGE_KEY = "predictiq.admin.apiKey";

/**
 * Cheap authenticated endpoint used to validate the entered admin key.
 * `GET /api/v1/admin/api-keys` requires ApiKeyAuth and returns a small payload,
 * so a 2xx confirms the key is valid without mutating anything.
 */
const VALIDATION_PATH = "/api/v1/admin/api-keys";

export default function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const [apiKey, setApiKey] = useState<string>("");
  const [unlocked, setUnlocked] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState<boolean>(false);

  useEffect(() => {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored) {
      setApiKey(stored);
    }
  }, []);

  async function handleUnlock(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setChecking(true);

    try {
      const origin = resolveApiOrigin();
      const response = await fetch(`${origin}${VALIDATION_PATH}`, {
        method: "GET",
        headers: {
          "X-API-Key": apiKey,
        },
      });

      if (!response.ok) {
        setUnlocked(false);
        setError(
          response.status === 401
            ? "Invalid API key. Please check the key and try again."
            : `Key validation failed (HTTP ${response.status}). Please try again.`,
        );
        return;
      }

      window.localStorage.setItem(STORAGE_KEY, apiKey);
      setUnlocked(true);
    } catch (err) {
      setUnlocked(false);
      setError(
        err instanceof Error
          ? `Could not reach the API: ${err.message}`
          : "Could not reach the API to validate the key.",
      );
    } finally {
      setChecking(false);
    }
  }

  if (unlocked) {
    return <>{children}</>;
  }

  return (
    <div className="flex min-h-screen items-center justify-center p-6">
      <form
        onSubmit={handleUnlock}
        className="w-full max-w-sm space-y-4 rounded-lg border p-6"
      >
        <h1 className="text-lg font-semibold">Admin access</h1>
        <p className="text-sm text-gray-600">
          Enter your admin API key to continue.
        </p>
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="API key"
          autoComplete="off"
          className="w-full rounded border px-3 py-2"
          required
        />
        {error ? (
          <p role="alert" className="text-sm text-red-600">
            {error}
          </p>
        ) : null}
        <button
          type="submit"
          disabled={checking || apiKey.length === 0}
          className="w-full rounded bg-black px-3 py-2 text-white disabled:opacity-50"
        >
          {checking ? "Validating…" : "Unlock"}
        </button>
      </form>
    </div>
  );
}
