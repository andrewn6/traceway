/**
 * Traceway + Vercel AI SDK example
 *
 * This example shows how to instrument Vercel AI SDK calls
 * so that traces and spans appear in your Traceway dashboard.
 *
 * Prerequisites:
 *   npm install ai @ai-sdk/openai traceway @opentelemetry/api @opentelemetry/sdk-trace-base
 *
 * Run:
 *   TRACEWAY_API_KEY=tw_sk_... OPENAI_API_KEY=sk-... npx tsx examples/vercel_ai_sdk.ts
 */

import { generateText } from 'ai';
import { openai } from '@ai-sdk/openai';
import { initTraceway } from 'traceway/ai';

// Initialize Traceway telemetry.
// Reads TRACEWAY_URL (typically http://localhost:4000 in local dev) and TRACEWAY_API_KEY from env.
const { tracer, shutdown } = initTraceway({
  // url: 'https://api.traceway.ai',   // or set TRACEWAY_URL
  // apiKey: 'tw_sk_...',               // or set TRACEWAY_API_KEY
  debug: true,
});

async function main() {
  // --- Simple text generation ---
  console.log('--- generateText ---');
  const result = await generateText({
    model: openai('gpt-4o-mini'),
    prompt: 'Explain what LLM observability is in 2 sentences.',
    experimental_telemetry: {
      isEnabled: true,
      tracer,
      functionId: 'explain-observability',
      metadata: {
        source: 'example-script',
      },
    },
  });
  console.log(result.text);
  console.log(`Tokens: ${result.usage.promptTokens} in / ${result.usage.completionTokens} out\n`);

  // --- Multi-step with tools ---
  console.log('--- generateText with tool ---');
  const toolResult = await generateText({
    model: openai('gpt-4o-mini'),
    prompt: 'What is the weather in San Francisco?',
    tools: {
      getWeather: {
        description: 'Get weather for a city',
        parameters: {
          type: 'object' as const,
          properties: {
            city: { type: 'string', description: 'City name' },
          },
          required: ['city'],
        },
        execute: async ({ city }: { city: string }) => {
          // Simulated weather lookup
          return { city, temperature: 62, condition: 'foggy' };
        },
      },
    },
    maxSteps: 3,
    experimental_telemetry: {
      isEnabled: true,
      tracer,
      functionId: 'weather-lookup',
    },
  });
  console.log(toolResult.text);

  // Flush all pending spans before exit
  await shutdown();
  console.log('\nDone — check your Traceway dashboard!');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
