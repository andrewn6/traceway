import Link from 'next/link';

export default function HomePage() {
  return (
    <div className="flex flex-col justify-center text-center flex-1 gap-4 px-4">
      <h1 className="text-3xl font-bold">Traceway Documentation</h1>
      <p className="text-fd-muted-foreground max-w-lg mx-auto">
        Open-source observability for LLM applications. Record traces and spans,
        build datasets, run evaluations, and understand what your models are
        doing.
      </p>
      <div className="flex gap-3 justify-center mt-2">
        <Link
          href="/docs"
          className="px-4 py-2 rounded-md bg-fd-primary text-fd-primary-foreground font-medium text-sm"
        >
          Get Started
        </Link>
        <Link
          href="/docs/sdk"
          className="px-4 py-2 rounded-md border border-fd-border font-medium text-sm"
        >
          SDK Reference
        </Link>
      </div>
    </div>
  );
}
