import { invoke } from '@tauri-apps/api/core';
import type { ApiResponse } from '../shared/types';

type LogLevel = 'INFO' | 'WARN' | 'ERROR';

export async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const startedAt = nowMs();
  void writeClientLog('INFO', 'api', `开始调用命令：${command}`, args);
  try {
    const response = await invoke<ApiResponse<T>>(command, args);
    const durationMs = Math.round(nowMs() - startedAt);
    if (!response.success) {
      void writeClientLog('ERROR', 'api', `命令返回失败：${command}`, {
        durationMs,
        error: response.error,
        args
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
        args
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
        details: details == null ? undefined : safeStringify(details)
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
