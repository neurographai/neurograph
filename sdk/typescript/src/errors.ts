// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

/** Base error for all NeuroGraph SDK errors. */
export class NeuroGraphError extends Error {
  public readonly code: string;
  public readonly statusCode?: number;

  constructor(message: string, code: string, statusCode?: number) {
    super(message);
    this.name = 'NeuroGraphError';
    this.code = code;
    this.statusCode = statusCode;
  }
}

/** Failed to connect to the NeuroGraph server. */
export class ConnectionError extends NeuroGraphError {
  constructor(url: string, cause?: string) {
    super(
      `Failed to connect to NeuroGraph at ${url}${cause ? `: ${cause}` : ''}`,
      'CONNECTION_ERROR',
    );
    this.name = 'ConnectionError';
  }
}

/** A query operation failed. */
export class QueryError extends NeuroGraphError {
  constructor(message: string, statusCode?: number) {
    super(message, 'QUERY_ERROR', statusCode);
    this.name = 'QueryError';
  }
}

/** Request timed out. */
export class TimeoutError extends NeuroGraphError {
  constructor(timeoutMs: number) {
    super(`Request timed out after ${timeoutMs}ms`, 'TIMEOUT');
    this.name = 'TimeoutError';
  }
}
