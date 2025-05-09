<script lang="ts">
	import Icon from "@iconify/svelte";

	interface Props {
		label: string;
		onClick: () => void;
        textSize?: string;
		className?: string;
		icon?: string;
		iconPosition?: "left" | "right";
		type?: string;
	}

	let {
		label,
		className,
		icon,
		iconPosition = "left",
		type = "quaternary",
		onClick,
	}: Props = $props();

	const typeClasses = {
		primary: "bg-primary hover:bg-primary/70 active:bg-primary/50",
		secondary: "bg-secondary hover:bg-secondary/70 active:bg-secondary/50 text-black",
		tertiary: "bg-tertiary hover:bg-tertiary/70 active:bg-tertiary/50",
		quaternary: "bg-quaternary hover:bg-tertiary/60 active:bg-tertiary/30",
	};

	const bgClass = typeClasses[type as keyof typeof typeClasses] || typeClasses.quaternary;
</script>

<button
	class="flex items-center gap-2 px-8 py-3 {bgClass} rounded-lg transition-colors w-full text-xl {className}"
	onclick={onClick}
>
	{#if icon && iconPosition === "left"}
		<Icon {icon} width={18} />
	{/if}

	<span class="flex-1 text-center">{label}</span>

	{#if icon && iconPosition === "right"}
		<Icon {icon} width={18} />
	{/if}
</button>
