import type { View } from './types';

export function newest<T extends { revision: number }>(current: T | null, incoming: T): T {
  return current && current.revision > incoming.revision ? current : incoming;
}

export function mergeView(current: View | null, incoming: View): View {
  return { ...incoming, client: newest(current?.client ?? null, incoming.client),
    updater: newest(current?.updater ?? null, incoming.updater) };
}
