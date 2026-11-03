import { spawn } from 'child_process';
import path from 'path';

const toolName = process.argv[2] || 'get_session_viewer';
let toolArgs = {};

if (process.argv[3]) {
  try {
    toolArgs = JSON.parse(process.argv[3]);
  } catch {
    console.error('Invalid JSON arguments passed.');
    process.exit(1);
  }
}

console.log(`\n🔍 Calling Air Selangor MCP Tool: [${toolName}]`);
if (Object.keys(toolArgs).length > 0) {
  console.log(`   With arguments:`, toolArgs);
}

const child = spawn(process.execPath, [path.resolve(process.cwd(), 'dist/index.js')], {
  stdio: ['pipe', 'pipe', 'inherit'],
});

const rpcRequest = {
  jsonrpc: '2.0',
  id: Date.now(),
  method: 'tools/call',
  params: {
    name: toolName,
    arguments: toolArgs,
  },
};

let buffer = '';

child.stdout.on('data', (chunk) => {
  buffer += chunk.toString();
  const lines = buffer.split('\n');
  buffer = lines.pop() || '';

  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const response = JSON.parse(line);
      if (response.result?.content) {
        console.log('\n✅ Response:');
        console.log(response.result.content[0].text);
      } else if (response.error) {
        console.error('\n❌ RPC Error:', response.error);
      } else {
        console.log('\n📦 Output:', response);
      }
      child.kill();
      process.exit(0);
    } catch {
      // Non-JSON log line
      console.log(line);
    }
  }
});

child.stdin.write(JSON.stringify(rpcRequest) + '\n');
