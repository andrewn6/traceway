/**
 * Traceway MCP Client
 * 
 * Use with Claude Code, Cursor, or any MCP-compatible client.
 * 
 * Setup in claude_desktop_config.json:
 * {
 *   "mcpServers": {
 *     "traceway": {
 *       "command": "npx",
 *       "args": ["@traceway/mcp"]
 *     }
 *   }
 * }
 * 
 * Or configure via environment:
 * - TRACEWAY_MCP_URL: MCP server URL (default: http://localhost:4000/v1/mcp)
 * - TRACEWAY_API_KEY: API key for authentication
 */

export interface TracewayMcpOptions {
  url?: string;
  apiKey?: string;
}

export interface TraceSummary {
  id: string;
  name?: string | null;
  status: 'failed' | 'running' | 'completed';
  duration_ms: number | null;
  total_tokens: number;
  total_cost: number;
  started_at: string;
  span_count: number;
  tags?: string[];
}

export interface SpanDetail {
  id: string;
  trace_id: string;
  name: string;
  kind: string;
  status: string;
  started_at: string;
  ended_at?: string | null;
  input?: unknown;
  output?: unknown;
}

export interface SearchResult {
  content: string;
  structuredContent?: {
    traces?: TraceSummary[];
    count?: number;
    query?: string;
  };
}

export interface TagResult {
  content: string;
  structuredContent?: {
    trace?: {
      id: string;
      tags?: string[];
    };
  };
}

export interface DatasetResult {
  content: string;
  structuredContent?: {
    datapoint?: {
      id: string;
    };
  };
}

export class TracewayMcpClient {
  private url: string;
  private apiKey?: string;

  constructor(options: TracewayMcpOptions = {}) {
    this.url = (
      options.url ?? 
      (typeof process !== 'undefined' ? process.env?.TRACEWAY_MCP_URL : undefined) ?? 
      'http://localhost:4000/v1/mcp'
    ).replace(/\/$/, '');
    
    this.apiKey = 
      options.apiKey ?? 
      (typeof process !== 'undefined' ? process.env?.TRACEWAY_API_KEY : undefined);
  }

  private async rpc<T>(method: string, params: unknown, id: number): Promise<T> {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.apiKey) headers['Authorization'] = `Bearer ${this.apiKey}`;

    const res = await fetch(this.url, {
      method: 'POST',
      headers,
      body: JSON.stringify({ jsonrpc: '2.0', id, method, params }),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`MCP ${method}: HTTP ${res.status} - ${text}`);
    }

    const body = await res.json() as { result?: T; error?: { code: number; message: string } };
    
    if (body.error) {
      throw new Error(`MCP ${method}: ${body.error.code} - ${body.error.message}`);
    }
    
    if (body.result === undefined) {
      throw new Error(`MCP ${method}: no result returned`);
    }
    
    return body.result;
  }

  /**
   * Initialize connection to the MCP server
   */
  async initialize(): Promise<{ serverInfo: { name: string; version: string }; capabilities: Record<string, unknown> }> {
    return this.rpc('initialize', {
      protocolVersion: '2024-11-05',
      capabilities: {},
      clientInfo: { name: 'traceway-mcp-client', version: '0.1.0' },
    }, 1);
  }

  /**
   * List available MCP tools
   */
  async listTools(): Promise<{ name: string; description: string }[]> {
    const result = await this.rpc<{ tools: { name: string; description: string }[] }>('tools/list', {}, 2);
    return result.tools;
  }

  /**
   * Search traces using query DSL
   * 
   * Query syntax:
   * - kind:llm_call - Filter by span kind
   * - status:failed - Filter by status (failed, running, completed)
   * - model:gpt-4 - Filter by model name
   * - since:24h - Time filter (s=seconds, m=minutes, h=hours, d=days)
   * - name:search - Filter by trace/span name
   * - tag:session:abc - Filter by tag
   * - Text terms - Full-text search
   */
  async searchTraces(query: string, limit = 20): Promise<SearchResult> {
    return this.rpc<SearchResult>('tools/call', {
      name: 'search_traces',
      arguments: { query, limit },
    }, 3);
  }

  /**
   * List the most recent traces
   */
  async listRecentTraces(limit = 10): Promise<SearchResult> {
    return this.rpc<SearchResult>('tools/call', {
      name: 'list_recent_traces',
      arguments: { limit },
    }, 4);
  }

  /**
   * Get full trace details as LLM-friendly text
   */
  async getTrace(traceId: string): Promise<{ content: string; structuredContent?: { trace: TraceSummary; spans: SpanDetail[] } }> {
    return this.rpc('tools/call', {
      name: 'get_trace',
      arguments: { trace_id: traceId },
    }, 5);
  }

  /**
   * Get span with full input/output
   */
  async getSpan(spanId: string): Promise<{ content: string; structuredContent?: { span: SpanDetail } }> {
    return this.rpc('tools/call', {
      name: 'get_span',
      arguments: { span_id: spanId },
    }, 6);
  }

  /**
   * Add tags to a trace (e.g., for organizing sessions)
   */
  async tagTrace(traceId: string, tags: string[]): Promise<TagResult> {
    return this.rpc<TagResult>('tools/call', {
      name: 'tag_trace',
      arguments: { trace_id: traceId, tags },
    }, 7);
  }

  /**
   * Add a span's input/output to a dataset for evaluation or fine-tuning
   */
  async addToDataset(datasetId: string, spanId: string): Promise<DatasetResult> {
    return this.rpc<DatasetResult>('tools/call', {
      name: 'add_to_dataset',
      arguments: { dataset_id: datasetId, span_id: spanId },
    }, 8);
  }
}

/**
 * Create a Traceway MCP client with default settings
 */
export function createMcpClient(options?: TracewayMcpOptions): TracewayMcpClient {
  return new TracewayMcpClient(options);
}

// Default export
export default TracewayMcpClient;
