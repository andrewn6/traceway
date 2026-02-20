<script lang="ts">
	import { signup } from '$lib/api';
	import { goto } from '$app/navigation';

	let email = $state('');
	let password = $state('');
	let name = $state('');
	let orgName = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';

		if (password.length < 8) {
			error = 'Password must be at least 8 characters';
			return;
		}

		loading = true;

		const result = await signup(email, password, name || undefined, orgName || undefined);

		if (result.ok) {
			goto('/');
		} else {
			error = result.error ?? 'Signup failed';
		}

		loading = false;
	}
</script>

<div class="min-h-screen flex items-center justify-center bg-bg">
	<div class="w-full max-w-sm space-y-6">
		<div class="text-center">
			<h1 class="text-2xl font-bold text-text">Traceway</h1>
			<p class="text-text-muted text-sm mt-1">Create your account</p>
		</div>

		<form onsubmit={handleSubmit} class="bg-bg-secondary border border-border rounded p-6 space-y-4">
			{#if error}
				<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">
					{error}
				</div>
			{/if}

			<div>
				<label for="name" class="block text-xs text-text-secondary mb-1">Name <span class="text-text-muted">(optional)</span></label>
				<input
					id="name"
					type="text"
					bind:value={name}
					autocomplete="name"
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
					placeholder="Your name"
				/>
			</div>

			<div>
				<label for="org-name" class="block text-xs text-text-secondary mb-1">Organization <span class="text-text-muted">(optional)</span></label>
				<input
					id="org-name"
					type="text"
					bind:value={orgName}
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
					placeholder="Your team or company"
				/>
			</div>

			<div>
				<label for="email" class="block text-xs text-text-secondary mb-1">Email</label>
				<input
					id="email"
					type="email"
					bind:value={email}
					required
					autocomplete="email"
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
					placeholder="you@example.com"
				/>
			</div>

			<div>
				<label for="password" class="block text-xs text-text-secondary mb-1">Password</label>
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

			<button
				type="submit"
				disabled={loading}
				class="w-full bg-accent text-bg font-semibold py-2 rounded text-sm hover:bg-accent/80 transition-colors disabled:opacity-50"
			>
				{loading ? 'Creating account...' : 'Create account'}
			</button>
		</form>

		<p class="text-center text-sm text-text-muted">
			Already have an account?
			<a href="/login" class="text-accent hover:underline">Sign in</a>
		</p>
	</div>
</div>
