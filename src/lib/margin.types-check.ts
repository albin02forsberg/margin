// Compile-time only (svelte-check): MarginApi in margin.d.ts must stay assignable both ways with the real `Margin`.
import type { Margin } from "./commands";
export const _toDecl: MarginApi = {} as Margin;
export const _fromDecl: Margin = {} as MarginApi;
