import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { registerTools } from './tools/index.js';
import { SteelManager } from './browser/steel.js';

async function main() {
  const server = new Server(
    {
      name: 'air-selangor-mcp',
      version: '1.0.0',
    },
    {
      capabilities: {
        tools: {},
      },
    }
  );

  const tools = registerTools();
  const toolMap = new Map(tools.map((t) => [t.name, t]));

  // List available tools
  server.setRequestHandler(ListToolsRequestSchema, async () => {
    return {
      tools: tools.map((t) => ({
        name: t.name,
        description: t.description,
        inputSchema: t.inputSchema,
      })),
    };
  });

  // Call tool handler
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    const tool = toolMap.get(name);

    if (!tool) {
      throw new Error(`Tool "${name}" not found.`);
    }

    try {
      const result = await tool.handler(args || {});
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2),
          },
        ],
      };
    } catch (error: any) {
      return {
        isError: true,
        content: [
          {
            type: 'text',
            text: `Error executing ${name}: ${error.message || String(error)}`,
          },
        ],
      };
    }
  });

  // Graceful shutdown
  const cleanup = async () => {
    try {
      const steelManager = SteelManager.getInstance();
      await steelManager.closeSession();
    } catch {
      // Ignore
    }
    process.exit(0);
  };

  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);

  // Connect via standard I/O transport
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('[AirSelangor MCP] Server running on stdio transport.');
}

main().catch((err) => {
  console.error('[AirSelangor MCP] Fatal error starting server:', err);
  process.exit(1);
});
