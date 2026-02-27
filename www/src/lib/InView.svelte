<script lang="ts">
	let { children, class: className = '', delay = 0, threshold = 0.15, once = true } = $props();
	let visible = $state(false);
	let el: HTMLDivElement;

	$effect(() => {
		if (!el) return;
		const observer = new IntersectionObserver(
			(entries) => {
				entries.forEach((entry) => {
					if (entry.isIntersecting) {
						setTimeout(() => { visible = true; }, delay);
						if (once) observer.unobserve(entry.target);
					} else if (!once) {
						visible = false;
					}
				});
			},
			{ threshold }
		);
		observer.observe(el);
		return () => observer.disconnect();
	});
</script>

<div
	bind:this={el}
	class="{className} transition-all duration-700 ease-out {visible ? 'opacity-100 translate-y-0 scale-100 blur-0' : 'opacity-0 translate-y-6 scale-[0.98] blur-[2px]'}"
>
	{@render children()}
</div>
