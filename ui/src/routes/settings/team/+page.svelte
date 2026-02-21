<script lang="ts">
	import {
		getOrgMembers,
		getInvites,
		createInvite,
		deleteInvite,
		type OrgMember,
		type InviteInfo
	} from '$lib/api';
	import { onMount } from 'svelte';

	let members: OrgMember[] = $state([]);
	let invites: InviteInfo[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	// Invite form
	let showInvite = $state(false);
	let inviteEmail = $state('');
	let inviteRole = $state('member');
	let sending = $state(false);
	let sent = $state(false);

	// Delete confirm
	let revokingId: string | null = $state(null);

	async function loadData() {
		try {
			const [m, i] = await Promise.all([getOrgMembers(), getInvites()]);
			members = m;
			invites = i;
		} catch {
			error = 'Failed to load team data';
		}
		loading = false;
	}

	async function handleInvite(e: Event) {
		e.preventDefault();
		if (!inviteEmail.trim()) return;
		sending = true;
		error = '';

		try {
			await createInvite(inviteEmail.trim(), inviteRole);
			sent = true;
			inviteEmail = '';
			showInvite = false;
			await loadData();
			setTimeout(() => (sent = false), 3000);
		} catch {
			error = 'Failed to send invite';
		}
		sending = false;
	}

	async function handleRevoke(id: string) {
		try {
			await deleteInvite(id);
			invites = invites.filter((i) => i.id !== id);
			revokingId = null;
		} catch {
			error = 'Failed to revoke invite';
		}
	}

	onMount(loadData);
</script>

<div class="max-w-3xl space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold">Team</h1>
		<button
			onclick={() => {
				showInvite = true;
				sent = false;
			}}
			class="px-3 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors"
		>
			Invite member
		</button>
	</div>

	<p class="text-text-muted text-sm">
		Manage your organization's team members and pending invitations.
	</p>

	{#if error}
		<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">
			{error}
		</div>
	{/if}

	{#if sent}
		<div class="bg-success/10 border border-success/30 rounded px-3 py-2 text-success text-sm">
			Invite sent successfully.
		</div>
	{/if}

	<!-- Invite form -->
	{#if showInvite}
		<form onsubmit={handleInvite} class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text">Invite a team member</h2>
			<div>
				<label for="invite-email" class="block text-xs text-text-secondary mb-1">Email address</label>
				<input
					id="invite-email"
					type="email"
					bind:value={inviteEmail}
					required
					placeholder="colleague@example.com"
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
				/>
			</div>
			<div>
				<label for="invite-role" class="block text-xs text-text-secondary mb-1">Role</label>
				<select
					id="invite-role"
					bind:value={inviteRole}
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text focus:outline-none focus:border-accent"
				>
					<option value="member">Member</option>
					<option value="admin">Admin</option>
				</select>
			</div>
			<div class="flex gap-2">
				<button
					type="submit"
					disabled={sending}
					class="px-4 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50"
				>
					{sending ? 'Sending...' : 'Send invite'}
				</button>
				<button
					type="button"
					onclick={() => (showInvite = false)}
					class="px-4 py-1.5 text-sm bg-bg-tertiary text-text rounded hover:bg-bg-secondary transition-colors"
				>
					Cancel
				</button>
			</div>
		</form>
	{/if}

	<!-- Members list -->
	<div class="space-y-2">
		<h2 class="text-sm font-semibold text-text-secondary uppercase tracking-wide">Members</h2>
		{#if loading}
			<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
		{:else if members.length === 0}
			<div class="text-text-muted text-sm py-8 text-center bg-bg-secondary border border-border rounded">
				No members yet.
			</div>
		{:else}
			<div class="bg-bg-secondary border border-border rounded divide-y divide-border">
				{#each members as member}
					<div class="px-4 py-3 flex items-center justify-between">
						<div class="space-y-0.5">
							<div class="text-sm font-medium text-text">
								{member.name || member.email}
							</div>
							<div class="text-xs text-text-muted">
								{member.email}
								<span class="ml-2 px-1.5 py-0.5 bg-bg-tertiary rounded text-xs">{member.role}</span>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Pending invites -->
	{#if !loading && invites.length > 0}
		<div class="space-y-2">
			<h2 class="text-sm font-semibold text-text-secondary uppercase tracking-wide">Pending Invites</h2>
			<div class="bg-bg-secondary border border-border rounded divide-y divide-border">
				{#each invites as invite}
					<div class="px-4 py-3 flex items-center justify-between">
						<div class="space-y-0.5">
							<div class="text-sm font-medium text-text">{invite.email}</div>
							<div class="text-xs text-text-muted">
								<span class="px-1.5 py-0.5 bg-bg-tertiary rounded">{invite.role}</span>
								<span class="ml-2">
									Expires {new Date(invite.expires_at).toLocaleDateString()}
								</span>
							</div>
						</div>
						<div>
							{#if revokingId === invite.id}
								<div class="flex items-center gap-2">
									<span class="text-xs text-text-muted">Revoke?</span>
									<button
										onclick={() => handleRevoke(invite.id)}
										class="text-xs text-danger hover:underline"
									>
										Yes
									</button>
									<button
										onclick={() => (revokingId = null)}
										class="text-xs text-text-muted hover:text-text"
									>
										No
									</button>
								</div>
							{:else}
								<button
									onclick={() => (revokingId = invite.id)}
									class="text-xs text-text-muted hover:text-danger transition-colors"
								>
									Revoke
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
