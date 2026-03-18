/**
 * Vercel AI SDK + Traceway example (curated TypeScript path).
 *
 * Run:
 *   TRACEWAY_API_KEY=tw_sk_... OPENAI_API_KEY=sk-... npx tsx examples/typescript/vercel_ai_sdk.ts
 */

import { generateText } from 'ai';
import { openai } from '@ai-sdk/openai';
import { initTraceway } from 'traceway/ai';

const { tracer, shutdown } = initTraceway({
  debug: true,
});

async function main() {
  const result = await generateText({
    model: openai('gpt-4o-mini'),
    prompt: 'Explain Traceway in one sentence.',
    experimental_telemetry: {
      isEnabled: true,
      tracer,
      functionId: 'curated-vercel-ai-sdk-example',
    },
  });

  console.log(result.text);
  await shutdown();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
