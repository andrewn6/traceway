<script lang="ts">
	import {
		getAllQueueItems,
		getDatasets,
		claimQueueItem,
		submitQueueItem,
		subscribeEvents,
		shortId,
		getAuthConfig,
		getAuthMe,
		getOrgMembers,
		type QueueItem,
		type DatasetWithCount,
	} from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import FloatingInspector from '$lib/components/FloatingInspector.svelte';
	import { onMount } from 'svelte';

	let items: QueueItem[] = $state([]);
	let datasets: DatasetWithCount[] = $state([]);
	let loading = $state(true);

	// Filter
	let statusFilter: 'all' | 'pending' | 'claimed' | 'completed' = $state('pending');

	// Reviewer identity (resolved from auth)
	let claimName = $state('reviewer');

	// Review mode
	let reviewingItem: QueueItem | null = $state(null);
	let editedJson = $state('');
	let submitting = $state(false);
	let inspectorWidth: 'compact' | 'default' | 'wide' = $state('default');
	let actionError = $state('');

	// Derived from reviewingItem for chat rendering
	const reviewingMessages = $derived(reviewingItem ? extractMessages(reviewingItem.original_data) : null);
	const reviewingExpected = $derived(reviewingItem ? extractExpectedOutput(reviewingItem.original_data) : null);

	// Helpers
	function formatDate(d: string): string {
		return new Date(d).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
		});
	}

	function formatJson(value: unknown): string {
		if (value === null || value === undefined) return '(none)';
		if (typeof value === 'string') return value;
		return JSON.stringify(value, null, 2);
	}

	function datasetName(datasetId: string): string {
		return datasets.find((d) => d.id === datasetId)?.name ?? shortId(datasetId);
	}

	// Detect chat messages in original_data for nicer rendering
	interface ChatMessage { role: string; content: string; }

	function extractMessages(data: unknown): ChatMessage[] | null {
		if (!data || typeof data !== 'object') return null;
		const obj = data as Record<string, unknown>;
		// DatapointKind::LlmConversation has { type: "llm_conversation", messages: [...] }
		if (obj.type === 'llm_conversation' && Array.isArray(obj.messages)) {
			return obj.messages as ChatMessage[];
		}
		// DatapointKind::Generic has { type: "generic", input: ... }
		// Try to extract messages from input if it's an array of {role, content}
		if (obj.type === 'generic' && obj.input) {
			if (Array.isArray(obj.input)) {
				const msgs = obj.input as ChatMessage[];
				if (msgs.length > 0 && msgs[0].role && msgs[0].content !== undefined) {
					return msgs;
				}
			}
			// OpenAI-style: { messages: [...] }
			if (typeof obj.input === 'object' && obj.input !== null) {
				const inp = obj.input as Record<string, unknown>;
				if (Array.isArray(inp.messages)) {
					const msgs = inp.messages as ChatMessage[];
					if (msgs.length > 0 && msgs[0].role) return msgs;
				}
			}
		}
		return null;
	}

	function extractExpectedOutput(data: unknown): unknown | null {
		if (!data || typeof data !== 'object') return null;
		const obj = data as Record<string, unknown>;
		if (obj.type === 'llm_conversation' && obj.expected) return obj.expected;
		if (obj.type === 'generic' && obj.expected_output) return obj.expected_output;
		return null;
	}

	// Counts
	const counts = $derived.by(() => {
		const pending = items.filter((i) => i.status === 'pending').length;
		const claimed = items.filter((i) => i.status === 'claimed').length;
		const completed = items.filter((i) => i.status === 'completed').length;
		return { pending, claimed, completed, total: pending + claimed + completed };
	});

	const filteredItems = $derived.by(() => {
		if (statusFilter === 'all') return items;
		return items.filter((i) => i.status === statusFilter);
	});

	async function load() {
		try {
			const [queueResult, dsResult] = await Promise.all([
				getAllQueueItems(),
				getDatasets(),
			]);
			items = queueResult.items;
			datasets = dsResult.datasets;
		} catch {
			// API not available
		}

		// Resolve reviewer name from auth
		try {
			const config = await getAuthConfig();
			if (config.mode === 'cloud') {
				const me = await getAuthMe();
				if (me.user_id) {
					const members = await getOrgMembers();
					const self = members.find((m) => m.id === me.user_id);
					if (self?.name) claimName = self.name;
					else if (self?.email) claimName = self.email;
				}
			}
		} catch {
			// Local mode or auth unavailable — keep default 'reviewer'
		}

		loading = false;
	}

	onMount(() => {
		load();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'queue_item_updated') {
				const existing = items.findIndex((i) => i.id === event.item.id);
				if (existing >= 0) {
					items[existing] = event.item;
					items = items;
				} else {
					items = [event.item, ...items];
				}
				// Update reviewing item if it's the one being updated
				if (reviewingItem?.id === event.item.id) {
					reviewingItem = event.item;
				}
			} else if (event.type === 'cleared') {
				items = [];
			}
		});

		return unsub;
	});

	async function handleClaim(itemId: string) {
		try {
			const updated = await claimQueueItem(itemId, claimName);
			items = items.map((i) => (i.id === updated.id ? updated : i));
			// Auto-open for review after claiming
			startReview(updated);
		} catch {
			// Conflict — already claimed
		}
	}

	function startReview(item: QueueItem) {
		reviewingItem = item;
		editedJson = JSON.stringify(item.original_data, null, 2);
		actionError = '';
	}

	function closeReview() {
		reviewingItem = null;
		editedJson = '';
		actionError = '';
	}

	async function handleApprove() {
		if (!reviewingItem) return;
		submitting = true;
		actionError = '';
		try {
			// Submit without edits = approve as-is
			const updated = await submitQueueItem(reviewingItem.id);
			items = items.map((i) => (i.id === updated.id ? updated : i));
			closeReview();
		} catch (err) {
			actionError = err instanceof Error ? err.message : 'Failed to approve item';
		}
		submitting = false;
	}

	async function handleSubmitEdited() {
		if (!reviewingItem) return;
		submitting = true;
		actionError = '';
		try {
			const data = JSON.parse(editedJson);
			const updated = await submitQueueItem(reviewingItem.id, data);
			items = items.map((i) => (i.id === updated.id ? updated : i));
			closeReview();
		} catch (err) {
			actionError = err instanceof Error ? err.message : 'Failed to submit edited item';
		}
		submitting = false;
	}

	function roleColor(role: string): string {
		switch (role.toLowerCase()) {
			case 'system': return 'border-warning/30 bg-warning/5';
			case 'user': return 'border-accent/30 bg-accent/5';
			case 'assistant': return 'border-success/30 bg-success/5';
			case 'tool': return 'border-purple-400/30 bg-purple-400/5';
			default: return 'border-border bg-bg-tertiary';
		}
	}

	function roleLabelColor(role: string): string {
		switch (role.toLowerCase()) {
			case 'system': return 'text-warning';
			case 'user': return 'text-accent';
			case 'assistant': return 'text-success';
			case 'tool': return 'text-purple-400';
			default: return 'text-text-muted';
		}
	}
</script>

<div class="app-shell-wide space-y-4">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold tracking-tight">Review</h1>
			<p class="text-xs text-text-muted mt-1">Human annotation queue across all datasets</p>
		</div>
	</div>

	<!-- Status filter tabs + counts -->
	<div class="app-toolbar-shell rounded-xl p-2 flex items-center gap-1.5 flex-wrap motion-rise-in">
		<button
			class="query-chip motion-interactive {statusFilter === 'pending' ? 'query-chip-active' : ''}"
			onclick={() => statusFilter = 'pending'}
		>
			Pending
			{#if counts.pending > 0}
				<span class="ml-1 px-1.5 py-0.5 rounded bg-warning/20 text-warning text-[10px]">{counts.pending}</span>
			{/if}
		</button>
		<button
			class="query-chip motion-interactive {statusFilter === 'claimed' ? 'query-chip-active' : ''}"
			onclick={() => statusFilter = 'claimed'}
		>
			Claimed
			{#if counts.claimed > 0}
				<span class="ml-1 px-1.5 py-0.5 rounded bg-accent/20 text-accent text-[10px]">{counts.claimed}</span>
			{/if}
		</button>
		<button
			class="query-chip motion-interactive {statusFilter === 'completed' ? 'query-chip-active' : ''}"
			onclick={() => statusFilter = 'completed'}
		>
			Completed
			{#if counts.completed > 0}
				<span class="ml-1 px-1.5 py-0.5 rounded bg-success/20 text-success text-[10px]">{counts.completed}</span>
			{/if}
		</button>
		<button
			class="query-chip motion-interactive {statusFilter === 'all' ? 'query-chip-active' : ''}"
			onclick={() => statusFilter = 'all'}
		>
			All
			<span class="ml-1 text-text-muted text-[10px]">{counts.total}</span>
		</button>
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-12">Loading...</div>
	{:else if filteredItems.length === 0}
		<div class="text-center py-16">
			<div class="text-text-muted text-sm">
				{#if statusFilter === 'pending'}
					No items waiting for review.
				{:else if statusFilter === 'claimed'}
					No items currently being reviewed.
				{:else if statusFilter === 'completed'}
					No completed reviews yet.
				{:else}
					Review queue is empty.
				{/if}
			</div>
			<div class="text-text-muted/50 text-xs mt-1">
				Items appear here when datapoints are enqueued from datasets, or when spans are sent to review from the trace viewer.
			</div>
		</div>
	{:else}
		<!-- Item list -->
		<div class="table-float divide-y divide-border/35 motion-rise-in">
			<div class="grid grid-cols-[80px_1fr_110px_92px_70px] gap-3 px-4 py-2 table-head-compact border-b border-border/55">
				<span>Dataset</span>
				<span>Preview</span>
				<span>Reviewer</span>
				<span class="text-right">Created</span>
				<span class="text-right">State</span>
			</div>
			{#each filteredItems as item (item.id)}
				<div
					class="w-full grid grid-cols-[80px_1fr_110px_92px_70px] gap-3 items-center px-4 py-2.5 cursor-pointer text-left motion-row
						{reviewingItem?.id === item.id
							? 'bg-accent/5'
							: 'hover:bg-bg-tertiary/20'}"
					role="row"
					tabindex="0"
					onclick={() => startReview(item)}
					onkeydown={(e) => { if (e.key === 'Enter') startReview(item); }}
				>
					<span class="text-[12px] text-text-muted truncate">
						{datasetName(item.dataset_id)}
					</span>

				<span class="text-[12px] text-text-secondary truncate font-mono">
					{#if extractMessages(item.original_data)}
						{@const messages = extractMessages(item.original_data)!}
						{messages[messages.length - 1].role}: {messages[messages.length - 1].content.slice(0, 100)}
					{:else}
						{JSON.stringify(item.original_data)?.slice(0, 100) ?? '(empty)'}
					{/if}
				</span>

					<span class="text-[11px] text-text-muted truncate">{item.claimed_by ?? '-'}</span>

					<span class="text-[11px] text-text-muted text-right">{formatDate(item.created_at)}</span>

					<div class="text-right">
					{#if item.status === 'pending'}
						<button
							type="button"
							class="px-2 py-1 text-[10px] bg-accent/10 text-accent border border-accent/20 rounded hover:bg-accent/20 transition-colors shrink-0"
							onclick={(e) => { e.stopPropagation(); handleClaim(item.id); }}
						>Claim</button>
					{:else if item.status === 'claimed'}
						<span class="px-2 py-0.5 text-[10px] text-accent border border-accent/20 rounded shrink-0">reviewing</span>
					{:else}
						<span class="px-2 py-0.5 text-[10px] text-success border border-success/20 rounded shrink-0">done</span>
					{/if}
					</div>
				</div>
			{/each}
		</div>

		<FloatingInspector
			open={!!reviewingItem}
			title={reviewingItem ? `Review ${shortId(reviewingItem.id)}` : 'Review'}
			subtitle={reviewingItem ? `Dataset: ${datasetName(reviewingItem.dataset_id)}` : ''}
			width={inspectorWidth}
			onclose={closeReview}
			onwidth={(w) => (inspectorWidth = w)}
		>
			{#if reviewingItem}
				<div class="space-y-3">
					{#if actionError}
						<div class="alert-danger text-[12px]">{actionError}</div>
					{/if}
					{#if reviewingMessages}
						<div class="space-y-2">
							<div class="text-xs text-text-muted uppercase mb-1">Conversation</div>
							{#each reviewingMessages as msg}
								<div class="rounded border px-3 py-2 {roleColor(msg.role)}">
									<div class="text-[10px] font-semibold uppercase mb-1 {roleLabelColor(msg.role)}">{msg.role}</div>
									<div class="text-sm text-text whitespace-pre-wrap">{msg.content}</div>
								</div>
							{/each}
						</div>
						{#if reviewingExpected}
							<div>
								<div class="text-xs text-text-muted uppercase mb-1">Expected Output</div>
								<pre class="text-xs query-float rounded p-3 overflow-auto max-h-40 font-mono text-text-secondary whitespace-pre-wrap">{formatJson(reviewingExpected)}</pre>
							</div>
						{/if}
					{:else}
						<div class="space-y-2">
							<div>
								<div class="text-xs text-text-muted uppercase mb-1">Original Data</div>
								<pre class="text-xs query-float rounded p-3 overflow-auto max-h-64 font-mono text-text-secondary whitespace-pre-wrap">{formatJson(reviewingItem.original_data)}</pre>
							</div>
							<div>
								<div class="text-xs text-text-muted uppercase mb-1">Edit (optional)</div>
								<textarea bind:value={editedJson} rows={10} class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-xs text-text font-mono resize-y"></textarea>
							</div>
						</div>
					{/if}

					<div class="flex items-center gap-2 pt-2 border-t border-border/60">
						{#if reviewingItem.status === 'pending'}
							<button class="btn-primary h-7 text-xs" onclick={() => handleClaim(reviewingItem.id)}>Claim & Start</button>
						{:else if reviewingItem.status === 'claimed'}
							<button class="btn-primary h-7 text-xs" onclick={handleApprove} disabled={submitting}>Approve</button>
							<button class="btn-secondary h-7 text-xs" onclick={handleSubmitEdited} disabled={submitting}>Submit edits</button>
						{/if}
						<div class="flex-1"></div>
						{#if reviewingItem.claimed_by}
							<span class="text-xs text-text-muted">Claimed by {reviewingItem.claimed_by}</span>
						{/if}
					</div>
				</div>
			{/if}
		</FloatingInspector>
	{/if}
</div>
