<script lang="ts">
	import { signup } from '$lib/api';
	import { goto } from '$app/navigation';

	let email = $state('');
	let password = $state('');
	let name = $state('');
	let orgName = $state('');
	let error = $state('');
	let loading = $state(false);
	let showPassword = $state(false);

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
			window.location.href = '/';
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
				<div class="relative">
					<input
						id="password"
						type={showPassword ? 'text' : 'password'}
						bind:value={password}
						required
						autocomplete="new-password"
						minlength="8"
						class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 pr-10 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
						placeholder="Min. 8 characters"
					/>
					<button
						type="button"
						onclick={() => showPassword = !showPassword}
						class="absolute right-2.5 top-1/2 -translate-y-1/2 text-text-muted hover:text-text transition-colors cursor-pointer"
						tabindex={-1}
					>
						{#if showPassword}
							<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 0 0 1.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.451 10.451 0 0 1 12 4.5c4.756 0 8.773 3.162 10.065 7.498a10.522 10.522 0 0 1-4.293 5.774M6.228 6.228 3 3m3.228 3.228 3.65 3.65m7.894 7.894L21 21m-3.228-3.228-3.65-3.65m0 0a3 3 0 1 0-4.243-4.243m4.242 4.242L9.88 9.88" /></svg>
						{:else}
							<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" /><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" /></svg>
						{/if}
					</button>
				</div>
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
