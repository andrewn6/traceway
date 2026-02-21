<script lang="ts">
	import { resetPassword } from '$lib/api';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let password = $state('');
	let confirmPassword = $state('');
	let error = $state('');
	let loading = $state(false);
	let success = $state(false);

	const token = $derived(page.url.searchParams.get('token') ?? '');
	const hasToken = $derived(token.length > 0);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';

		if (!hasToken) {
			error = 'Missing reset token. Please use the link from your email.';
			return;
		}

		if (password.length < 8) {
			error = 'Password must be at least 8 characters';
			return;
		}

		if (password !== confirmPassword) {
			error = 'Passwords do not match';
			return;
		}

		loading = true;

		const result = await resetPassword(token, password);

		if (result.ok) {
			success = true;
			setTimeout(() => goto('/login'), 2000);
		} else {
			error = result.message ?? 'Failed to reset password';
		}

		loading = false;
	}
</script>

<div class="min-h-screen flex items-center justify-center bg-bg">
	<div class="w-full max-w-sm space-y-6">
		<div class="text-center">
			<h1 class="text-2xl font-bold text-text">Traceway</h1>
			<p class="text-text-muted text-sm mt-1">Set a new password</p>
		</div>

		{#if !hasToken}
			<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">
				Invalid reset link. Please use the link from your email.
			</div>
		{:else if success}
			<div class="bg-bg-secondary border border-border rounded p-6">
				<div class="bg-success/10 border border-success/30 rounded px-3 py-2 text-success text-sm">
					Password reset successfully. Redirecting to sign in...
				</div>
			</div>
		{:else}
			<form onsubmit={handleSubmit} class="bg-bg-secondary border border-border rounded p-6 space-y-4">
				{#if error}
					<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">
						{error}
					</div>
				{/if}

				<div>
					<label for="password" class="block text-xs text-text-secondary mb-1">New password</label>
					<input
						id="password"
						type="password"
						bind:value={password}
						required
						autocomplete="new-password"
						minlength="8"
						class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
						placeholder="Min. 8 characters"
					/>
				</div>

				<div>
					<label for="confirm-password" class="block text-xs text-text-secondary mb-1">Confirm new password</label>
					<input
						id="confirm-password"
						type="password"
						bind:value={confirmPassword}
						required
						autocomplete="new-password"
						minlength="8"
						class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
						placeholder="Repeat your password"
					/>
				</div>

				<button
					type="submit"
					disabled={loading}
					class="w-full bg-accent text-bg font-semibold py-2 rounded text-sm hover:bg-accent/80 transition-colors disabled:opacity-50"
				>
					{loading ? 'Resetting...' : 'Reset password'}
				</button>
			</form>
		{/if}

		<p class="text-center text-sm text-text-muted">
			<a href="/login" class="text-accent hover:underline">Back to sign in</a>
		</p>
	</div>
</div>
