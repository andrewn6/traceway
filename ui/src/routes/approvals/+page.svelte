<script lang="ts">
	import {
		getAllQueueItems,
		getDatasets,
		getDatapoint,
		getSpan,
		claimQueueItem,
		submitQueueItem,
		subscribeEvents,
		shortId,
		type QueueItem,
		type DatasetWithCount,
		type Datapoint,
		type Span,
	} from '$lib/api';
	import { onMount } from 'svelte';

	let items: QueueItem[] = $state([]);
	let datasets: DatasetWithCount[] = $state([]);
	let loading = $state(true);

	let statusFilter: 'all' | 'pending' | 'claimed' | 'completed' = $state('pending');
	let savedView: 'all' | 'mine' | 'unclaimed' | 'completed_today' = $state('all');
	let sortBy: 'created_desc' | 'created_asc' | 'status' = $state('created_desc');
	let datasetFilter: string = $state('all');
	let searchQuery = $state('');
	let mineOnly = $state(false);
	let selectedItemId: string | null = $state(null);
	let detailOpen = $state(false);
	let selectedIds: Set<string> = $state(new Set());

	let selectedDatapoint: Datapoint | null = $state(null);
	let selectedSourceSpan: Span | null = $state(null);
	let loadingDetail = $state(false);

	let claimName = $state('reviewer');
	let editedJson = $state('');
	let submitting = $state(false);
	let actionError = $state('');
	let actionSuccess = $state('');

	const selectedItem = $derived(items.find((i) => i.id === selectedItemId) ?? null);

	const filteredItems = $derived.by(() => {
		const now = new Date();
		const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
		const filtered = items.filter((i) => {
			const statusOk = statusFilter === 'all' || i.status === statusFilter;
			const datasetOk = datasetFilter === 'all' || i.dataset_id === datasetFilter;
			const mineFlagOk = !mineOnly || i.claimed_by === claimName;
			const viewOk =
				savedView === 'all'
				|| (savedView === 'mine' && i.claimed_by === claimName)
				|| (savedView === 'unclaimed' && i.status === 'pending')
				|| (savedView === 'completed_today' && i.status === 'completed' && new Date(i.created_at).getTime() >= today);
			const q = searchQuery.trim().toLowerCase();
			const searchOk = q.length === 0
				|| i.id.toLowerCase().includes(q)
				|| i.datapoint_id.toLowerCase().includes(q)
				|| (i.claimed_by ?? '').toLowerCase().includes(q)
				|| datasetName(i.dataset_id).toLowerCase().includes(q);
			return statusOk && datasetOk && mineFlagOk && viewOk && searchOk;
		});

		filtered.sort((a, b) => {
			if (sortBy === 'created_asc') {
				return new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
			}
			if (sortBy === 'status') {
				const diff = statusRank(a.status) - statusRank(b.status);
				if (diff !== 0) return diff;
				return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
			}
			return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
		});

		return filtered;
	});

	const nextPending = $derived(items.find((i) => i.status === 'pending') ?? null);
	const nextClaimedByMe = $derived(items.find((i) => i.status === 'claimed' && i.claimed_by === claimName) ?? null);

	const counts = $derived.by(() => {
		const pending = items.filter((i) => i.status === 'pending').length;
		const claimed = items.filter((i) => i.status === 'claimed').length;
		const completed = items.filter((i) => i.status === 'completed').length;
		return { pending, claimed, completed, total: items.length };
	});

	function datasetName(datasetId: string): string {
		return datasets.find((d) => d.id === datasetId)?.name ?? shortId(datasetId);
	}

	function statusTone(status: QueueItem['status']): string {
		if (status === 'pending') return 'bg-warning/20 text-warning border-warning/30';
		if (status === 'claimed') return 'bg-accent/20 text-accent border-accent/30';
		return 'bg-success/20 text-success border-success/30';
	}

	function rowAccent(status: QueueItem['status']): string {
		if (status === 'pending') return 'border-l-warning/50';
		if (status === 'claimed') return 'border-l-accent/50';
		return 'border-l-success/50';
	}

	function statusFilterClass(status: typeof statusFilter): string {
		if (statusFilter !== status) return 'query-chip';
		if (status === 'pending') return 'query-chip border-warning/35 bg-warning/12 text-warning';
		if (status === 'claimed') return 'query-chip border-accent/35 bg-accent/12 text-accent';
		if (status === 'completed') return 'query-chip border-success/35 bg-success/12 text-success';
		return 'query-chip query-chip-active';
	}

	function statusRank(status: QueueItem['status']): number {
		if (status === 'pending') return 0;
		if (status === 'claimed') return 1;
		return 2;
	}

	const selectedVisibleCount = $derived.by(() => filteredItems.filter((i) => selectedIds.has(i.id)).length);
	const allVisibleSelected = $derived(filteredItems.length > 0 && selectedVisibleCount === filteredItems.length);

	function toggleSelected(id: string) {
		const next = new Set(selectedIds);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selectedIds = next;
	}

	function toggleSelectAllVisible() {
		if (allVisibleSelected) {
			selectedIds = new Set();
			return;
		}
		selectedIds = new Set(filteredItems.map((i) => i.id));
	}

	function formatDate(iso?: string | null): string {
		if (!iso) return '-';
		return new Date(iso).toLocaleString(undefined, {
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

	async function load() {
		loading = true;
		try {
			const [queueResult, dsResult] = await Promise.all([
				getAllQueueItems(),
				getDatasets(),
			]);
			items = queueResult.items;
			datasets = dsResult.datasets;
			if (!selectedItemId && items.length > 0) selectedItemId = items[0].id;
		} catch {
			items = [];
		} finally {
			loading = false;
		}
	}

	async function loadDetail() {
		if (!selectedItem) {
			selectedDatapoint = null;
			selectedSourceSpan = null;
			return;
		}
		loadingDetail = true;
		selectedDatapoint = null;
		selectedSourceSpan = null;
		try {
			const dp = await getDatapoint(selectedItem.dataset_id, selectedItem.datapoint_id);
			selectedDatapoint = dp;
			editedJson = JSON.stringify(selectedItem.edited_data ?? selectedItem.original_data ?? dp.kind, null, 2);
			if (dp.source_span_id) {
				selectedSourceSpan = await getSpan(dp.source_span_id).catch(() => null);
			}
		} catch {
			editedJson = JSON.stringify(selectedItem.edited_data ?? selectedItem.original_data ?? null, null, 2);
		} finally {
			loadingDetail = false;
		}
	}

	async function handleClaim(item: QueueItem) {
		actionError = '';
		try {
			const updated = await claimQueueItem(item.id, claimName);
			items = items.map((i) => (i.id === updated.id ? updated : i));
			selectedItemId = updated.id;
			detailOpen = true;
			await loadDetail();
			actionSuccess = `Claimed ${shortId(updated.id)}`;
		} catch {
			actionError = 'Could not claim item (it may already be claimed).';
		}
	}

	function selectNextOpenItem(currentId?: string | null) {
		const ordered = filteredItems;
		if (ordered.length === 0) {
			selectedItemId = null;
			return;
		}
		const idx = currentId ? ordered.findIndex((i) => i.id === currentId) : -1;
		const next = idx >= 0 ? ordered[idx + 1] : ordered[0];
		selectedItemId = (next ?? ordered[0]).id;
	}

	async function claimNextPending() {
		if (!nextPending) return;
		await handleClaim(nextPending);
	}

	async function bulkClaimSelected() {
		const targets = filteredItems.filter((i) => selectedIds.has(i.id) && i.status === 'pending');
		if (targets.length === 0) return;
		submitting = true;
		actionError = '';
		actionSuccess = '';
		try {
			for (const item of targets) {
				const updated = await claimQueueItem(item.id, claimName);
				items = items.map((i) => (i.id === updated.id ? updated : i));
			}
			actionSuccess = `Claimed ${targets.length} item${targets.length === 1 ? '' : 's'}`;
		} catch (err) {
			actionError = err instanceof Error ? err.message : 'Bulk claim failed';
		}
		submitting = false;
	}

	async function bulkApproveSelected() {
		const targets = filteredItems.filter((i) => selectedIds.has(i.id) && i.status !== 'completed');
		if (targets.length === 0) return;
		submitting = true;
		actionError = '';
		actionSuccess = '';
		let approved = 0;
		try {
			for (const item of targets) {
				let candidate = item;
				if (candidate.status === 'pending') {
					candidate = await claimQueueItem(candidate.id, claimName);
					items = items.map((i) => (i.id === candidate.id ? candidate : i));
				}
				const updated = await submitQueueItem(candidate.id);
				items = items.map((i) => (i.id === updated.id ? updated : i));
				approved += 1;
			}
			actionSuccess = `Approved ${approved} item${approved === 1 ? '' : 's'}`;
			selectedIds = new Set();
		} catch (err) {
			actionError = err instanceof Error ? err.message : 'Bulk approve failed';
		}
		submitting = false;
	}

	function openNextClaimedByMe() {
		if (!nextClaimedByMe) return;
		selectedItemId = nextClaimedByMe.id;
		detailOpen = true;
	}

	async function approveCurrent() {
		if (!selectedItem) return;
		submitting = true;
		actionError = '';
		actionSuccess = '';
		const currentId = selectedItem.id;
		try {
			let claimCandidate = selectedItem;
			if (selectedItem.status === 'pending') {
				claimCandidate = await claimQueueItem(selectedItem.id, claimName);
				items = items.map((i) => (i.id === claimCandidate.id ? claimCandidate : i));
			}
			const updated = await submitQueueItem(claimCandidate.id);
			items = items.map((i) => (i.id === updated.id ? updated : i));
			selectedItemId = updated.id;
			actionSuccess = `Approved ${shortId(updated.id)}`;
			selectNextOpenItem(currentId);
		} catch (err) {
			actionError = err instanceof Error ? err.message : 'Could not approve item.';
		}
		submitting = false;
	}

	async function submitEditedCurrent() {
		if (!selectedItem) return;
		submitting = true;
		actionError = '';
		actionSuccess = '';
		const currentId = selectedItem.id;
		try {
			const parsed = editedJson.trim() ? JSON.parse(editedJson) : null;
			let claimCandidate = selectedItem;
			if (selectedItem.status === 'pending') {
				claimCandidate = await claimQueueItem(selectedItem.id, claimName);
				items = items.map((i) => (i.id === claimCandidate.id ? claimCandidate : i));
			}
			const updated = await submitQueueItem(claimCandidate.id, parsed);
			items = items.map((i) => (i.id === updated.id ? updated : i));
			selectedItemId = updated.id;
			actionSuccess = `Submitted ${shortId(updated.id)}`;
			selectNextOpenItem(currentId);
		} catch (err) {
			actionError = err instanceof Error ? err.message : 'Could not submit edited payload.';
		}
		submitting = false;
	}

	function moveSelection(delta: 1 | -1) {
		if (filteredItems.length === 0) return;
		if (!selectedItemId) {
			selectedItemId = filteredItems[0].id;
			return;
		}
		const idx = filteredItems.findIndex((i) => i.id === selectedItemId);
		if (idx < 0) {
			selectedItemId = filteredItems[0].id;
			return;
		}
		const nextIdx = Math.max(0, Math.min(filteredItems.length - 1, idx + delta));
		selectedItemId = filteredItems[nextIdx].id;
		detailOpen = true;
	}

	function isTypingTarget(target: EventTarget | null): boolean {
		const el = target as HTMLElement | null;
		if (!el) return false;
		if (el.isContentEditable) return true;
		const tag = el.tagName;
		return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
	}

	function onKeydown(e: KeyboardEvent) {
		if (isTypingTarget(e.target)) return;
		if (e.key === 'j') {
			e.preventDefault();
			moveSelection(1);
			return;
		}
		if (e.key === 'k') {
			e.preventDefault();
			moveSelection(-1);
			return;
		}
		if (!selectedItem || submitting) return;
		if (e.key === 'a' && selectedItem.status === 'claimed') {
			e.preventDefault();
			approveCurrent();
			return;
		}
		if (e.key === 'e' && selectedItem.status === 'claimed') {
			e.preventDefault();
			submitEditedCurrent();
		}
		if (e.key === 'Escape' && detailOpen) {
			e.preventDefault();
			detailOpen = false;
		}
	}

	$effect(() => {
		if (selectedItemId && !items.some((i) => i.id === selectedItemId)) {
			selectedItemId = null;
		}
	});

	$effect(() => {
		const visible = new Set(filteredItems.map((i) => i.id));
		const next = new Set<string>();
		for (const id of selectedIds) {
			if (visible.has(id)) next.add(id);
		}
		if (next.size !== selectedIds.size) selectedIds = next;
	});

	$effect(() => {
		if (selectedItem) {
			actionError = '';
			actionSuccess = '';
			loadDetail();
		}
	});

	$effect(() => {
		load();
	});

	onMount(() => {
		document.addEventListener('keydown', onKeydown);
		const unsub = subscribeEvents((event) => {
			if (event.type === 'queue_item_updated') {
				const idx = items.findIndex((i) => i.id === event.item.id);
				if (idx >= 0) {
					items[idx] = event.item;
					items = items;
				} else {
					items = [event.item, ...items];
				}
			}
		});
		return () => {
			document.removeEventListener('keydown', onKeydown);
			unsub();
		};
	});
</script>

<div class="app-shell-wide space-y-4">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-lg font-semibold text-text">Approvals</h1>
			<p class="text-xs text-text-muted mt-0.5">Trace-linked human approval queue for model and agent outputs</p>
		</div>
	</div>

	<div class="app-toolbar-shell rounded-xl p-2 space-y-2">
		<div class="flex items-center gap-1.5 flex-wrap">
			<button class="query-chip {savedView === 'all' ? 'query-chip-active' : ''}" onclick={() => (savedView = 'all')}>All queue</button>
			<button class="query-chip {savedView === 'mine' ? 'query-chip-active' : ''}" onclick={() => (savedView = 'mine')}>My queue</button>
			<button class="query-chip {savedView === 'unclaimed' ? 'query-chip-active' : ''}" onclick={() => (savedView = 'unclaimed')}>Unclaimed</button>
			<button class="query-chip {savedView === 'completed_today' ? 'query-chip-active' : ''}" onclick={() => (savedView = 'completed_today')}>Completed today</button>
			<div class="flex-1"></div>
			<select bind:value={sortBy} class="control-select h-8 text-[12px] w-[170px]">
				<option value="created_desc">Newest first</option>
				<option value="created_asc">Oldest first</option>
				<option value="status">Status priority</option>
			</select>
		</div>

		<div class="flex items-center gap-1.5 flex-wrap">
			<button class={statusFilterClass('pending')} onclick={() => (statusFilter = 'pending')}>Pending ({counts.pending})</button>
			<button class={statusFilterClass('claimed')} onclick={() => (statusFilter = 'claimed')}>Claimed ({counts.claimed})</button>
			<button class={statusFilterClass('completed')} onclick={() => (statusFilter = 'completed')}>Completed ({counts.completed})</button>
			<button class={statusFilterClass('all')} onclick={() => (statusFilter = 'all')}>All ({counts.total})</button>
			<div class="flex-1"></div>
			<div class="text-[12px] text-text-muted">{filteredItems.length} items</div>
		</div>

		<div class="grid grid-cols-1 md:grid-cols-[220px_minmax(0,1fr)_150px_auto_auto_auto] gap-2 items-center">
			<select bind:value={datasetFilter} class="control-select h-8 text-[12px]">
				<option value="all">All datasets</option>
				{#each datasets as ds (ds.id)}
					<option value={ds.id}>{ds.name}</option>
				{/each}
			</select>
			<input class="control-input h-8 text-[12px]" bind:value={searchQuery} placeholder="Search item, dataset, reviewer" />
			<input class="control-input h-8 text-[12px]" bind:value={claimName} placeholder="Reviewer" />
			<button class="query-chip h-8 {mineOnly ? 'query-chip-active' : ''}" onclick={() => (mineOnly = !mineOnly)}>Mine</button>
			<button class="btn-secondary h-8 text-[12px]" disabled={!nextPending} onclick={claimNextPending}>Claim next</button>
			<button class="btn-ghost h-8 text-[12px]" disabled={!nextClaimedByMe} onclick={openNextClaimedByMe}>Open next</button>
		</div>

		{#if selectedVisibleCount > 0}
			<div class="flex items-center gap-2 text-[12px] border border-border/50 rounded-lg px-2 py-1.5 bg-bg-tertiary/30">
				<span class="text-text-muted">{selectedVisibleCount} selected</span>
				<button class="btn-secondary h-7 text-[11px]" disabled={submitting} onclick={bulkClaimSelected}>Bulk claim</button>
				<button class="btn-primary h-7 text-[11px]" disabled={submitting} onclick={bulkApproveSelected}>Bulk approve</button>
				<button class="btn-ghost h-7 text-[11px]" onclick={() => (selectedIds = new Set())}>Clear</button>
			</div>
		{/if}
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-10">Loading approvals...</div>
	{:else}
		<div class="table-float overflow-hidden">
			{#if filteredItems.length === 0}
				<div class="px-3 py-8 text-center text-sm text-text-muted">No items match current filters.</div>
			{:else}
				<div class="max-h-[min(72vh,980px)] overflow-y-auto">
					<div class="grid grid-cols-[34px_100px_1fr_170px_130px_110px_120px] gap-3 px-3 py-2 table-head-compact border-b border-border/55 sticky top-0 z-10 bg-bg-secondary/96 backdrop-blur-sm">
						<label class="flex items-center justify-center"><input type="checkbox" checked={allVisibleSelected} onchange={toggleSelectAllVisible} class="accent-accent" /></label>
						<span>Status</span>
						<span>Dataset / Item</span>
						<span>Datapoint</span>
						<span>Created</span>
						<span>Reviewer</span>
						<span class="text-right">Action</span>
					</div>
					{#each filteredItems as item (item.id)}
						<button type="button" class="w-full text-left grid grid-cols-[34px_100px_1fr_170px_130px_110px_120px] gap-3 items-center px-3 py-2 border-b border-border/45 border-l-2 {rowAccent(item.status)} hover:bg-bg-secondary/45 motion-row cursor-pointer" onclick={() => { selectedItemId = item.id; detailOpen = true; }}>
							<div class="flex items-center justify-center">
								<input type="checkbox" checked={selectedIds.has(item.id)} onclick={(e) => e.stopPropagation()} onchange={() => toggleSelected(item.id)} class="accent-accent" />
							</div>
							<div><span class={`px-2 py-0.5 rounded text-[11px] border ${statusTone(item.status)}`}>{item.status}</span></div>
							<div class="min-w-0">
								<div class="text-[13px] text-text truncate">{datasetName(item.dataset_id)}</div>
								<div class="text-[11px] text-text-muted font-mono truncate">{shortId(item.id)}</div>
							</div>
							<div class="text-[12px] font-mono text-text-secondary truncate">{shortId(item.datapoint_id)}</div>
							<div class="text-[12px] text-text-muted">{formatDate(item.created_at)}</div>
							<div class="text-[12px] text-text-muted truncate">{item.claimed_by ?? '-'}</div>
							<div class="text-right">
								<span class="btn-secondary h-7 text-[11px] inline-flex items-center">Review</span>
							</div>
						</button>
					{/each}
				</div>
			{/if}
		</div>

		{#if detailOpen && selectedItem}
			<button type="button" class="fixed inset-0 z-40 bg-black/35" onclick={() => (detailOpen = false)} aria-label="Close approval review modal"></button>
			<div class="fixed left-1/2 -translate-x-1/2 top-[9.5rem] bottom-[4.25rem] w-[min(920px,calc(100vw-2rem))] z-50 table-float rounded-xl overflow-y-auto motion-rise-in">
				<div class="px-3 py-2 border-b border-border/55 bg-gradient-to-r from-bg-secondary/75 via-bg-secondary/45 to-transparent sticky top-0 z-10 backdrop-blur-md flex items-center justify-between gap-2">
					<p class="text-[11px] uppercase tracking-[0.14em] text-text-muted">Approval review</p>
					<button class="text-[11px] px-2 py-1 border border-border rounded-md text-text-muted hover:text-text hover:bg-bg-tertiary/70 transition-colors duration-150" onclick={() => (detailOpen = false)}>Close</button>
				</div>
				<div class="p-3.5 space-y-3">
					{#if loadingDetail}
						<div class="h-24 flex items-center justify-center text-[12px] text-text-muted">Loading linked datapoint...</div>
					{:else}
						{#if actionError}<div class="alert-danger text-[12px]">{actionError}</div>{/if}
						{#if actionSuccess}<div class="alert-success text-[12px]">{actionSuccess}</div>{/if}

						<div>
							<div class="text-[14px] font-semibold text-text">{datasetName(selectedItem.dataset_id)}</div>
							<div class="text-[11px] text-text-muted font-mono">queue:{shortId(selectedItem.id)} datapoint:{shortId(selectedItem.datapoint_id)}</div>
						</div>

						<div class="grid grid-cols-2 gap-2 text-[12px]">
							<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Status</div><div class="text-text mt-0.5 capitalize">{selectedItem.status}</div></div>
							<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Reviewer</div><div class="text-text mt-0.5">{selectedItem.claimed_by ?? '-'}</div></div>
							<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Created</div><div class="text-text mt-0.5">{formatDate(selectedItem.created_at)}</div></div>
							<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Claimed</div><div class="text-text mt-0.5">{formatDate(selectedItem.claimed_at)}</div></div>
						</div>

						{#if selectedSourceSpan}
							<div class="surface-quiet border border-accent/20 px-2.5 py-2 text-[12px]">
								<div class="text-text-muted">Source Trace</div>
								<a href={`/traces/${selectedSourceSpan.trace_id}`} class="text-accent hover:underline font-mono">trace/{shortId(selectedSourceSpan.trace_id)} • span/{shortId(selectedSourceSpan.id)}</a>
								<div class="text-[11px] text-text-muted mt-1">{selectedSourceSpan.name}</div>
							</div>
						{/if}

						<div class="space-y-1.5">
							<div class="label-micro uppercase">Original Data</div>
							<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap max-h-[180px] overflow-auto">{formatJson(selectedItem.original_data ?? selectedDatapoint?.kind)}</pre>
						</div>

						<div class="space-y-1.5">
							<div class="label-micro uppercase">Edited / Decision Payload</div>
							<textarea class="control-textarea text-[12px] font-mono min-h-[150px]" bind:value={editedJson}></textarea>
						</div>

						<div class="flex items-center gap-2">
							{#if selectedItem.status === 'pending'}
								<button class="btn-secondary" disabled={submitting} onclick={() => handleClaim(selectedItem)}>Claim</button>
								<button class="btn-primary" disabled={submitting} onclick={approveCurrent}>{submitting ? 'Submitting...' : 'Claim + approve'}</button>
							{/if}
							{#if selectedItem.status === 'claimed'}
								<button class="btn-secondary" disabled={submitting} onclick={approveCurrent}>{submitting ? 'Submitting...' : 'Approve as is'}</button>
								<button class="btn-primary" disabled={submitting} onclick={submitEditedCurrent}>{submitting ? 'Submitting...' : 'Submit edited'}</button>
							{/if}
							{#if selectedItem.status === 'completed'}
								<div class="text-[12px] text-success">This item is completed.</div>
							{/if}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	{/if}
</div>
