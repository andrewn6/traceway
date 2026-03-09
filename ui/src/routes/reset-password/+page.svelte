<script lang="ts">
	import { resetPassword } from '$lib/api';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import TracewayWordmark from '$lib/components/TracewayWordmark.svelte';

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
			<a href="/" class="inline-flex items-center justify-center px-3 h-10 rounded-lg border border-border/65 bg-bg-secondary/65 hover:border-border transition-colors">
				<TracewayWordmark className="h-4.5 w-auto text-text" />
			</a>
			<p class="text-text-muted text-sm mt-1">Set a new password</p>
		</div>

		{#if !hasToken}
			<div class="alert-danger">
				Invalid reset link. Please use the link from your email.
			</div>
		{:else if success}
			<div class="auth-card">
				<div class="alert-success">
					Password reset successfully. Redirecting to sign in...
				</div>
			</div>
		{:else}
			<form onsubmit={handleSubmit} class="auth-card space-y-4">
				{#if error}
					<div class="alert-danger">
						{error}
					</div>
				{/if}

				<div>
					<label for="password" class="label-micro block mb-1">New password</label>
					<input
						id="password"
						type="password"
						bind:value={password}
						required
						autocomplete="new-password"
						minlength="8"
						class="control-input"
						placeholder="Min. 8 characters"
					/>
				</div>

				<div>
					<label for="confirm-password" class="label-micro block mb-1">Confirm new password</label>
					<input
						id="confirm-password"
						type="password"
						bind:value={confirmPassword}
						required
						autocomplete="new-password"
						minlength="8"
						class="control-input"
						placeholder="Repeat your password"
					/>
				</div>

				<button
					type="submit"
					disabled={loading}
					class="btn-primary w-full"
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
