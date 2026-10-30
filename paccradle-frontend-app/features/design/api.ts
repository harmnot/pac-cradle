import { apiJson, HttpMethod, type ApiResult } from "@/lib/api-client";
import { none, some, type Maybe } from "@/lib/maybe";
import type { Design } from "./types";

/**
 * Saved designs ("My designs", contracts `designs-2026-10-30` and `billing-me-2026-10-30`).
 * The engine's own design JSON lives in `./types`.
 */

/** Insert styles a saved design can have. */
export const InsertStyle = {
  PartitionGrid: "partition_grid",
  CutoutCard: "cutout_card",
  CradlePlatform: "cradle_platform",
} as const;

export type InsertStyle = (typeof InsertStyle)[keyof typeof InsertStyle];

/** How a design was unlocked for production files. */
export type UnlockSource = "purchase" | "credit" | "pro";

interface WireUnlock {
  readonly source: UnlockSource;
  readonly edits_until: string;
  readonly covers_current_version: boolean;
}

interface WireDesignSummary {
  readonly design_id: string;
  readonly name: string;
  readonly version: number;
  readonly insert_style: InsertStyle;
  readonly product_types: number;
  readonly product_count: number;
  readonly has_box: boolean;
  readonly created_at: string;
  readonly updated_at: string;
  readonly unlock: WireUnlock | null;
}

interface WirePage<T> {
  readonly items: readonly T[];
  readonly page: number;
  readonly page_size: number;
  readonly total: number;
  readonly total_pages: number;
}

/** A design's unlock, as a badge shows it. */
export interface Unlock {
  readonly source: UnlockSource;
  readonly editsUntil: string;
  /** False once the design changed after its 30-day edit window: exporting pays again. */
  readonly coversCurrentVersion: boolean;
}

/** One "My designs" item. */
export interface DesignSummary {
  readonly id: string;
  readonly name: string;
  readonly version: number;
  readonly style: InsertStyle;
  readonly productCount: number;
  readonly hasBox: boolean;
  readonly updatedAt: string;
  readonly unlock: Maybe<Unlock>;
}

/** A page of designs. */
export interface DesignPage {
  readonly items: readonly DesignSummary[];
  readonly total: number;
  readonly totalPages: number;
}

/** Sort columns "My designs" offers. */
export type DesignSort = "updated_at" | "created_at" | "name";

/** What page of designs to read. */
export interface DesignQuery {
  readonly page: number;
  readonly pageSize: number;
  readonly sort: DesignSort;
  readonly order: "asc" | "desc";
}

/** The query string of `query`, also its cache key. */
export function designQueryString(query: DesignQuery): string {
  return new URLSearchParams({
    page: String(query.page),
    page_size: String(query.pageSize),
    sort: query.sort,
    order: query.order,
  }).toString();
}

function toUnlock(wire: WireUnlock | null): Maybe<Unlock> {
  return wire === null
    ? none
    : some({
        source: wire.source,
        editsUntil: wire.edits_until,
        coversCurrentVersion: wire.covers_current_version,
      });
}

function toSummary(wire: WireDesignSummary): DesignSummary {
  return {
    id: wire.design_id,
    name: wire.name,
    version: wire.version,
    style: wire.insert_style,
    productCount: wire.product_count,
    hasBox: wire.has_box,
    updatedAt: wire.updated_at,
    unlock: toUnlock(wire.unlock),
  };
}

/** A page of the signed-in person's designs (a guest's too). */
export async function listDesigns({
  query,
  signal,
}: {
  query: DesignQuery;
  signal?: AbortSignal;
}): Promise<ApiResult<DesignPage>> {
  const result = await apiJson<WirePage<WireDesignSummary>>({
    path: `/designs?${designQueryString(query)}`,
    signal,
  });
  if (result.kind === "error") {
    return result;
  }
  return {
    ...result,
    value: {
      items: result.value.items.map(toSummary),
      total: result.value.total,
      totalPages: result.value.total_pages,
    },
  };
}

/** `POST /designs`' 201 and `GET /designs/{design_id}`: a design with its spec. */
interface WireDesign {
  readonly design_id: string;
  readonly name: string;
  readonly version: number;
  readonly spec: Design;
  readonly created_at: string;
  readonly updated_at: string;
  /** Left out of a new design's answer; `null` until it is paid for (M1-D8). */
  readonly unlock?: WireUnlock | null;
}

/** A saved design: the engine's spec as stored, and the version to send back with a change. */
export interface SavedDesign {
  readonly id: string;
  readonly name: string;
  readonly version: number;
  readonly spec: Design;
  readonly updatedAt: string;
  readonly unlock: Maybe<Unlock>;
}

function toSaved(wire: WireDesign): SavedDesign {
  return {
    id: wire.design_id,
    name: wire.name,
    version: wire.version,
    spec: wire.spec,
    updatedAt: wire.updated_at,
    unlock: toUnlock(wire.unlock ?? null),
  };
}

/** Most characters a design's name may have (the API trims it and refuses more). */
export const DESIGN_NAME_MAX = 200;

/**
 * Saves a new design at version 1 (contract `designs-2026-10-30`). Guests may save; their
 * designs move to the account when they sign up or in. `DESIGN_INVALID` means the spec broke
 * a rule the WASM preview runs too.
 */
export async function createDesign({
  name,
  spec,
}: {
  name: string;
  spec: Design;
}): Promise<ApiResult<SavedDesign>> {
  const result = await apiJson<WireDesign>({
    method: HttpMethod.Post,
    path: "/designs",
    body: { name, spec },
  });
  return result.kind === "ok" ? { ...result, value: toSaved(result.value) } : result;
}

/** The caller's design (`DESIGN_NOT_FOUND` for anyone else's). */
export async function getDesign({
  id,
  signal,
}: {
  id: string;
  signal?: AbortSignal;
}): Promise<ApiResult<SavedDesign>> {
  const result = await apiJson<WireDesign>({
    path: `/designs/${encodeURIComponent(id)}`,
    signal,
  });
  return result.kind === "ok" ? { ...result, value: toSaved(result.value) } : result;
}
