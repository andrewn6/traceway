<script lang="ts">
	import { forgotPassword } from '$lib/api';
	import TracewayWordmark from '$lib/components/TracewayWordmark.svelte';

	let email = $state('');
	let error = $state('');
	let loading = $state(false);
	let submitted = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		const result = await forgotPassword(email);

		if (result.ok) {
			submitted = true;
		} else {
			error = result.message ?? 'Something went wrong';
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
			<p class="text-text-muted text-sm mt-1">Reset your password</p>
		</div>

		{#if submitted}
			<div class="auth-card space-y-3">
				<div class="alert-success">
					Check your email for a reset link.
				</div>
				<p class="text-text-muted text-sm">
					If an account exists for <strong class="text-text">{email}</strong>, we've sent a password reset link. The link expires in 1 hour.
				</p>
			</div>
		{:else}
			<form onsubmit={handleSubmit} class="auth-card space-y-4">
				{#if error}
					<div class="alert-danger">
						{error}
					</div>
				{/if}

				<p class="text-text-muted text-sm">
					Enter your email address and we'll send you a link to reset your password.
				</p>

				<div>
					<label for="email" class="label-micro block mb-1">Email</label>
					<input
						id="email"
						type="email"
						bind:value={email}
						required
						autocomplete="email"
						class="control-input"
						placeholder="you@example.com"
					/>
				</div>

				<button
					type="submit"
					disabled={loading}
					class="btn-primary w-full"
				>
					{loading ? 'Sending...' : 'Send reset link'}
				</button>
			</form>
		{/if}

		<p class="text-center text-sm text-text-muted">
			Remember your password?
			<a href="/login" class="text-accent hover:underline">Sign in</a>
		</p>
	</div>
</div>
