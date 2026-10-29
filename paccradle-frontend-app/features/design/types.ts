/**
 * The geometry engine's JSON (`insert_core`, CLAUDE.md §6): the design the editor sends
 * and what the engine returns. Field names and shapes follow the Rust types exactly; all
 * lengths are millimetres, x right and y down the page, z up.
 *
 * Optional values are `null` here because that is how the engine writes them; read them
 * at the edges and pass `Maybe` values (lib/maybe.ts) around the UI.
 */

/** A point in mm, `[x, y]`. */
export type Point = readonly [number, number];

/** A closed outline; the last point joins the first. */
export type Polygon = readonly Point[];

/** A straight line between two points: hinge axes. */
export interface Line {
  readonly from: Point;
  readonly to: Point;
}

/** The axis-aligned box round a feature. */
export interface Bounds {
  readonly min: Point;
  readonly max: Point;
}

// ---------------------------------------------------------------- design (input)

export type Board = "paperboard" | "chipboard" | "e_flute" | "b_flute" | "c_flute";

/** A board and the numbers the generators use for it (CLAUDE.md §6.4). */
export interface Material {
  readonly board: Board;
  readonly thickness_mm: number;
  readonly clearance_mm: number;
  readonly slot_tolerance_mm: number;
}

/** One product type: a top-view outline from its top-left corner, its height and count. */
export interface Product {
  readonly id: string;
  readonly contour_mm: Polygon;
  readonly height_mm: number;
  readonly quantity: number;
}

export type Align =
  | "top_left"
  | "top"
  | "top_right"
  | "left"
  | "center"
  | "right"
  | "bottom_left"
  | "bottom"
  | "bottom_right";

/** Where one product sits on a card or deck, as dragged in the editor. */
export interface Placement {
  readonly product_id: string;
  readonly center_mm: Point;
  readonly rotation_deg: number;
}

export type NotchSide = "bottom" | "top" | "both";

/** Where a hole's finger notch goes: which long side, and how far along (0–1). */
export interface FingerNotches {
  readonly side: NotchSide;
  readonly along: number;
}

export interface PartitionGridSpec {
  readonly style: "partition_grid";
  readonly rows: number;
  readonly columns: number;
  readonly divider_height_mm: number;
  readonly outer_frame: boolean;
  readonly finger_notches: boolean;
}

export interface CutoutCardSpec {
  readonly style: "cutout_card";
  readonly margin_mm: number;
  readonly finger_notches: FingerNotches | null;
  readonly align: Align;
  readonly placements: readonly Placement[];
}

export interface CradlePlatformSpec {
  readonly style: "cradle_platform";
  readonly deck_height_mm: number;
  readonly margin_mm: number;
  readonly align: Align;
  readonly placements: readonly Placement[];
}

/** The insert style and its settings, tagged by `style`. */
export type InsertSpec = PartitionGridSpec | CutoutCardSpec | CradlePlatformSpec;

export type BoxStyle = "mailer" | "shipping";

export interface BoxSize {
  readonly length_mm: number;
  readonly width_mm: number;
  readonly height_mm: number;
}

/** The outer box: its style and board, and a fixed inside size or `null` to fit the insert. */
export interface BoxSpec {
  readonly style: BoxStyle;
  readonly material: Material;
  readonly inside_mm: BoxSize | null;
}

/** Everything needed to generate one insert and its outer box. */
export interface Design {
  readonly material: Material;
  readonly products: readonly Product[];
  readonly insert: InsertSpec;
  readonly outer_box: BoxSpec | null;
}

// ---------------------------------------------------------------- packaging (output)

/** One piece of a path, tagged by `kind`. Arcs turn clockwise on screen when positive. */
export type Segment =
  | { readonly kind: "line"; readonly to: Point }
  | { readonly kind: "arc"; readonly center: Point; readonly sweep_deg: number };

/** A cut, crease or perforation line. The preview's paths are straight lines only. */
export interface Path {
  readonly start: Point;
  readonly segments: readonly Segment[];
  readonly closed: boolean;
}

/** One flat face of a piece, for the 3D fold. */
export interface Panel {
  readonly outline: Polygon;
  readonly holes: readonly Polygon[];
}

export type FoldDirection = "valley" | "mountain";

/** A crease that folds panel `child` against panel `parent` (CLAUDE.md §7.2). */
export interface Hinge {
  readonly parent: number;
  readonly child: number;
  readonly axis: Line;
  readonly fold_deg: number;
  readonly direction: FoldDirection;
  /** How far past the crease the child turns (a bend round another layer of board). */
  readonly offset_mm: number;
  /** Fold order for the animation: lower stages fold first. */
  readonly stage: number;
}

/** One flat part to cut. Panel 0 is the root of the fold. */
export interface Piece {
  readonly name: string;
  readonly quantity: number;
  readonly outline: Path;
  readonly holes: readonly Path[];
  readonly creases: readonly Path[];
  readonly perforations: readonly Path[];
  readonly panels: readonly Panel[];
  readonly hinges: readonly Hinge[];
}

export type Stance = "flat" | "upright_along_x" | "upright_along_y";

/** Where one cut copy of a piece goes in the assembled insert or box. */
export interface Mount {
  readonly piece: number;
  readonly stance: Stance;
  readonly at_mm: readonly [number, number, number];
}

/** A generated insert: its pieces, footprint, where each product sits and each piece goes. */
export interface Insert {
  readonly pieces: readonly Piece[];
  readonly length_mm: number;
  readonly width_mm: number;
  readonly height_mm: number;
  readonly placements: readonly Placement[];
  readonly mounts: readonly Mount[];
}

/** A generated outer box. */
export interface OuterBox {
  readonly style: BoxStyle;
  readonly pieces: readonly Piece[];
  readonly inside_mm: BoxSize;
  readonly outside_mm: BoxSize;
  readonly mounts: readonly Mount[];
}

/** Everything one design generates. */
export interface Packaging {
  readonly insert: Insert;
  readonly outer_box: OuterBox | null;
}

// ---------------------------------------------------------------- fit check

export type FitColor = "green" | "yellow" | "red";

/** What a red product runs into, tagged by `kind`. */
export type Clash =
  | { readonly kind: "board" }
  | { readonly kind: "product"; readonly placement: number }
  | { readonly kind: "leg" }
  | { readonly kind: "off_insert" };

/** One product copy's fit (CLAUDE.md §7.3). */
export interface ProductFit {
  /** Index into `Insert.placements`. */
  readonly placement: number;
  readonly color: FitColor;
  readonly clearance_mm: number;
  readonly target_mm: number;
  readonly clashes: readonly Clash[];
}

// ---------------------------------------------------------------- warnings

/** A sheet or cutting mat; a piece fits either way round. */
export interface SheetSize {
  readonly length_mm: number;
  readonly width_mm: number;
}

/** What to check against beyond the design. */
export interface CheckOptions {
  readonly sheet: SheetSize | null;
}

export interface MmRange {
  readonly min: number;
  readonly max: number;
}

/** Which manufacturability rule fired, with the numbers behind it, tagged by `kind`. */
export type Rule =
  | { readonly kind: "holes_too_close"; readonly gap_mm: number; readonly min_mm: number }
  | { readonly kind: "hole_near_edge"; readonly gap_mm: number; readonly min_mm: number }
  | { readonly kind: "hole_near_crease"; readonly gap_mm: number; readonly min_mm: number }
  | { readonly kind: "slot_too_tight"; readonly slot_mm: number; readonly board_mm: number }
  | { readonly kind: "slot_too_loose"; readonly slot_mm: number; readonly board_mm: number }
  | {
      readonly kind: "too_small";
      readonly feature: "board" | "cut_out";
      readonly size_mm: number;
      readonly min_mm: number;
    }
  | { readonly kind: "crease_crosses_cut" }
  | { readonly kind: "open_cut" }
  | { readonly kind: "duplicate_line"; readonly length_mm: number }
  | {
      readonly kind: "too_big_for_sheet";
      readonly length_mm: number;
      readonly width_mm: number;
      readonly sheet: SheetSize;
    }
  | { readonly kind: "weak_panel"; readonly cut_out_share: number; readonly max_share: number }
  | {
      readonly kind: "product_below_deck";
      readonly product_id: string;
      readonly height_mm: number;
      readonly deck_height_mm: number;
    }
  | { readonly kind: "product_over_leg"; readonly product_id: string }
  | {
      readonly kind: "unusual_thickness";
      readonly board: Board;
      readonly thickness_mm: number;
      readonly typical: MmRange;
    };

/** Where a warning points: the insert or box, a piece, a spot on it, the products involved. */
export interface Spot {
  readonly part: "insert" | "outer_box";
  readonly piece: number | null;
  readonly area_mm: Bounds | null;
  readonly placements: readonly number[];
}

/** One thing to fix before cutting (CLAUDE.md §6.5). */
export interface Warning {
  readonly rule: Rule;
  readonly at: Spot;
}
