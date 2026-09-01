/**
 * useToast — shared design-system notification primitive (#1319).
 *
 * A module-level store (no Context/Provider wiring required) so the
 * imperative `toast.*` API below can be called from anywhere — not just
 * components — including API mutation handlers for bet placement, market
 * creation, subscription confirmation, and key rotation. Mount a single
 * `<ToastContainer />` (see Toast.tsx) once, near the app root, and every
 * `toast.success(...)` / `toast.error(...)` call anywhere in the tree will
 * render into it.
 */

import { useSyncExternalStore } from 'react';

export type ToastVariant = 'success' | 'error' | 'warning' | 'info';

export interface ToastItem {
  id: string;
  variant: ToastVariant;
  message: string;
  /** ms before auto-dismiss; 0 disables auto-dismiss (e.g. for errors). */
  duration: number;
}

type Listener = () => void;

const DEFAULT_DURATION: Record<ToastVariant, number> = {
  success: 4000,
  info: 4000,
  warning: 6000,
  error: 0,
};

let toasts: ToastItem[] = [];
const listeners = new Set<Listener>();
const timers = new Map<string, ReturnType<typeof setTimeout>>();
let nextId = 0;

function emit() {
  for (const listener of listeners) listener();
}

function dismiss(id: string) {
  const timer = timers.get(id);
  if (timer) {
    clearTimeout(timer);
    timers.delete(id);
  }
  toasts = toasts.filter((t) => t.id !== id);
  emit();
}

function show(variant: ToastVariant, message: string, durationMs?: number): string {
  const id = `toast-${++nextId}`;
  const duration = durationMs ?? DEFAULT_DURATION[variant];
  toasts = [...toasts, { id, variant, message, duration }];
  emit();

  if (duration > 0) {
    timers.set(
      id,
      setTimeout(() => dismiss(id), duration)
    );
  }

  return id;
}

export const toast = {
  success: (message: string, durationMs?: number) => show('success', message, durationMs),
  error: (message: string, durationMs?: number) => show('error', message, durationMs),
  warning: (message: string, durationMs?: number) => show('warning', message, durationMs),
  info: (message: string, durationMs?: number) => show('info', message, durationMs),
  dismiss,
};

function subscribe(listener: Listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSnapshot() {
  return toasts;
}

function getServerSnapshot(): ToastItem[] {
  return [];
}

/** Read-only view of the current toast queue, for rendering `<ToastContainer />`. */
export function useToast() {
  const items = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
  return { toasts: items, dismiss };
}
