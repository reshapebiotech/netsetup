import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import type { ComponentProps } from "svelte";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// These types are heavily used by shadcn-svelte components
export type WithElementRef<T> = T & {
	ref?: HTMLElement | null;
};

export type WithoutChild<T> = Omit<T, "child">;
