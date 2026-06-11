import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse } from '../shared/types';

type LogLevel = 'INFO' | 'WARN' | 'ERROR';

export async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const startedAt = nowMs();
  void writeClientLog('INFO', 'api', `开始调用命令：${command}`, {
    stage: 'start',
    args: summarizeArgs(args)
  });
  try {
    const response = await invoke<ApiResponse<T>>(command, args);
    const durationMs = Math.round(nowMs() - startedAt);
    if (!response.success) {
      void writeClientLog('ERROR', 'api', `命令返回失败：${command}`, {
        durationMs,
        error: response.error,
        args: summarizeArgs(args)
      });
      const error = new Error(response.error?.message ?? '操作失败') as Error & {
        code?: string;
        details?: unknown;
      };
      error.code = response.error?.code;
      error.details = response.error?.details;
      throw error;
    }
    void writeClientLog('INFO', 'api', `命令调用成功：${command}`, {
      durationMs,
      result: summarizeResult(response.data)
    });
    return response.data as T;
  } catch (error) {
    if (!(error instanceof Error && 'code' in error)) {
      void writeClientLog('ERROR', 'api', `命令调用异常：${command}`, {
        durationMs: Math.round(nowMs() - startedAt),
        error,
        args: summarizeArgs(args)
      });
    }
    throw error;
  }
}

export function isTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function writeClientLog(
  level: LogLevel,
  module: string,
  message: string,
  details?: unknown
) {
  if (!isTauriRuntime()) {
    return;
  }
  try {
    await invoke<ApiResponse<boolean>>('write_client_log', {
      payload: {
        level,
        module,
        message,
        details: details == null ? undefined : safeStringify(sanitizeLogDetails(details))
      }
    });
  } catch (error) {
    console.warn('write_client_log failed', error);
  }
}

function nowMs() {
  return globalThis.performance?.now?.() ?? Date.now();
}

function summarizeResult(data: unknown) {
  if (Array.isArray(data)) {
    return { type: 'array', count: data.length };
  }
  if (data && typeof data === 'object') {
    const record = data as Record<string, unknown>;
    return {
      type: 'object',
      id: record.id,
      orderNo: record.orderNo,
      keys: Object.keys(record).slice(0, 12)
    };
  }
  return data;
}

function summarizeArgs(args?: Record<string, unknown>) {
  if (!args) {
    return { type: 'none', keys: [] as string[] };
  }
  return {
    type: 'object',
    keys: Object.keys(args).slice(0, 20)
  };
}

function sanitizeLogDetails(value: unknown) {
  const seen = new WeakSet<object>();

  const visit = (current: unknown, key?: string): unknown => {
    if (current instanceof Error) {
      return {
        name: current.name,
        message: sanitizeText(current.message),
        stack: current.stack ? sanitizeText(current.stack) : undefined
      };
    }

    if (typeof current === 'string') {
      return sanitizeStringByKey(key, current);
    }

    if (Array.isArray(current)) {
      if (isLineCollectionKey(key)) {
        return summarizeCollection(current);
      }
      return current.map((item) => visit(item, key));
    }

    if (current && typeof current === 'object') {
      if (seen.has(current)) {
        return '[Circular]';
      }
      seen.add(current);

      const entries = Object.entries(current as Record<string, unknown>);
      if (isLineCollectionKey(key)) {
        return {
          type: 'object',
          keys: entries.map(([entryKey]) => entryKey).slice(0, 20)
        };
      }

      return Object.fromEntries(entries.map(([entryKey, entryValue]) => [
        entryKey,
        visit(entryValue, entryKey)
      ]));
    }

    return current;
  };

  try {
    return visit(value);
  } catch {
    return '[LogDetailsRedacted]';
  }
}

function summarizeCollection(items: unknown[]) {
  const fieldNames = new Set<string>();
  for (const item of items.slice(0, 5)) {
    if (item && typeof item === 'object' && !Array.isArray(item)) {
      for (const key of Object.keys(item)) {
        fieldNames.add(key);
      }
    }
  }
  return {
    type: 'array',
    count: items.length,
    fields: Array.from(fieldNames).slice(0, 20)
  };
}

function sanitizeStringByKey(key: string | undefined, value: string) {
  const normalizedKey = key?.toLowerCase() ?? '';
  if (isPhoneKey(normalizedKey)) {
    return redactPhone(value);
  }
  if (isAddressKey(normalizedKey)) {
    return redactAddress(value);
  }
  if (isNameKey(normalizedKey)) {
    return redactName(value);
  }
  if (isPathKey(normalizedKey)) {
    return finalPathComponent(value);
  }
  return sanitizeText(value);
}

function sanitizeText(value: string) {
  return redactPhone(redactPathLikeText(value));
}

function isPhoneKey(key: string) {
  return key.includes('phone') || key.includes('tel') || key.includes('mobile');
}

function isAddressKey(key: string) {
  return key.includes('address') || key.includes('addr');
}

function isNameKey(key: string) {
  return key === 'name' || key.includes('customername') || key.includes('suppliername') || key.includes('merchantname');
}

function isPathKey(key: string) {
  return key.includes('path') || key.includes('file') || key.includes('dir');
}

function isLineCollectionKey(key?: string) {
  const normalizedKey = key?.toLowerCase() ?? '';
  return ['items', 'lines', 'details'].some((itemKey) => normalizedKey.includes(itemKey));
}

function redactPhone(value: string) {
  return value.replace(/\d{7,}/g, (match) => {
    if (match.length <= 5) {
      return '***';
    }
    return `${match.slice(0, 3)}***${match.slice(-2)}`;
  });
}

function redactAddress(value: string) {
  const trimmed = value.trim();
  if (trimmed.length <= 4) {
    return '***';
  }
  return `${trimmed.slice(0, 4)}***`;
}

function redactName(value: string) {
  const trimmed = value.trim();
  if (trimmed.length <= 2) {
    return '***';
  }
  return `${trimmed.slice(0, 1)}*${trimmed.slice(-1)}`;
}

function redactPathLikeText(value: string) {
  return value.replace(/[A-Za-z]:[\\/][^\s,;，；|]+/g, (match) => finalPathComponent(match));
}

function finalPathComponent(value: string) {
  const normalized = value.trim().replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : normalized;
}

function safeStringify(value: unknown) {
  const seen = new WeakSet<object>();
  let text: string;
  try {
    text = JSON.stringify(value, (_key, current) => {
      if (current instanceof Error) {
        return {
          name: current.name,
          message: current.message,
          stack: current.stack
        };
      }
      if (current && typeof current === 'object') {
        if (seen.has(current)) {
          return '[Circular]';
        }
        seen.add(current);
      }
      return current;
    });
  } catch (error) {
    text = String(error);
  }
  if (text.length > 1800) {
    return `${text.slice(0, 1800)}...<truncated>`;
  }
  return text;
}
