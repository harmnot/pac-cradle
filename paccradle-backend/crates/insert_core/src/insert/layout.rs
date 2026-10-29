//! Where every product copy goes on a card or deck: the design's own placements stay as
//! they are, and copies without one are laid straight in rows (shelf packing).

use super::hole::{Lie, ProductHole};
use crate::design::{Align, Placement, Product};
use crate::error::GenerateError;
use crate::geometry::{Area, Bounds, Point, Transform};

/// Slack when checking that rows fit a fixed space, for floating-point noise.
pub(super) const FIT_SLACK_MM: f64 = 1e-6;
/// Row lengths closer than this give the same rows; only one is tried.
const SAME_ROW_LENGTH_MM: f64 = 0.01;
/// Most row lengths tried for a growing card. Keeps big quantities fast.
const MAX_ROW_LENGTH_TRIES: usize = 256;
/// Stop collecting row lengths after this many (before removing repeats).
const MAX_ROW_LENGTHS_COLLECTED: usize = 20_000;

/// What to lay out.
pub(super) struct LayoutRequest<'a> {
    pub products: &'a [Product],
    /// One hole per product, in the same order.
    pub holes: &'a [ProductHole],
    /// The design's own placements.
    pub placements: &'a [Placement],
    /// Board to leave between two holes.
    pub gap_mm: f64,
    /// Where the generated rows must fit: a fixed card or deck minus its insets. `None`
    /// lets the board grow around the products.
    pub space: Option<Bounds>,
    /// Where the block of rows sits in `space`, and how rows line up inside the block.
    pub align: Align,
}

/// Every product copy's placement and hole, the design's own placements first.
#[derive(Default)]
pub(super) struct Layout {
    pub placements: Vec<Placement>,
    pub holes: Vec<Area>,
    /// The box around the holes the generator placed. `None` when the design placed every copy.
    pub generated_bounds: Option<Bounds>,
}

impl Layout {
    /// The box around every hole.
    pub(super) fn bounds(&self) -> Option<Bounds> {
        bounds_of(&self.holes)
    }

    /// The board of fixed `size` (length along x), if the rows the generator laid out
    /// fit in its room.
    pub(super) fn fixed_board(
        &self,
        (size, insets): ((f64, f64), Insets),
    ) -> Result<Bounds, GenerateError> {
        let room = insets.room(size);
        let has_room = room.length_mm() > 0.0 && room.width_mm() > 0.0;
        let rows_fit = self
            .generated_bounds
            .is_none_or(|rows| room.expanded(FIT_SLACK_MM).encloses(rows));
        if has_room && rows_fit {
            return Ok(Bounds::from_size(size));
        }
        let needed = self
            .bounds()
            .map_or((0.0, 0.0), |bounds| (bounds.length_mm(), bounds.width_mm()));
        Err(GenerateError::DoesNotFit {
            needed_length_mm: needed.0 + 2.0 * insets.x_mm,
            needed_width_mm: needed.1 + 2.0 * insets.y_mm,
            room_length_mm: size.0,
            room_width_mm: size.1,
        })
    }

    /// A board just big enough for the holes plus the insets, with everything moved so
    /// the board's top-left corner is at (0, 0).
    pub(super) fn grown_board(mut self, insets: Insets) -> (Bounds, Self) {
        let holes = self
            .bounds()
            .unwrap_or_else(|| Bounds::from_size((0.0, 0.0)));
        let board = Bounds::from_size((
            holes.length_mm() + 2.0 * insets.x_mm,
            holes.width_mm() + 2.0 * insets.y_mm,
        ));
        let shift = Point::new(insets.x_mm, insets.y_mm) - holes.min;
        for placement in &mut self.placements {
            placement.center_mm = placement.center_mm + shift;
        }
        for hole in &mut self.holes {
            *hole = hole.transformed(Transform::shift(shift));
        }
        (board, self)
    }
}

/// Board kept clear between the holes and the edges of a card or deck.
#[derive(Debug, Clone, Copy)]
pub(super) struct Insets {
    /// From the left and right edges.
    pub x_mm: f64,
    /// From the top and bottom edges.
    pub y_mm: f64,
}

impl Insets {
    /// The same inset from every edge.
    pub(super) const fn even(mm: f64) -> Self {
        Self { x_mm: mm, y_mm: mm }
    }

    /// Where holes may go on a board of `size` (length along x) with its top-left corner
    /// at (0, 0).
    pub(super) fn room(self, (length_mm, width_mm): (f64, f64)) -> Bounds {
        Bounds {
            min: Point::new(self.x_mm, self.y_mm),
            max: Point::new(length_mm - self.x_mm, width_mm - self.y_mm),
        }
    }
}

/// A copy with no placement yet.
struct Waiting<'a> {
    product_id: &'a str,
    hole: &'a ProductHole,
}

/// Where a block of rows starts, and how long a row may get.
#[derive(Debug, Clone, Copy)]
struct Shelf {
    /// Top-left corner of the block.
    origin: Point,
    row_limit_mm: f64,
    /// Align the block in this space (see [`LayoutRequest::align`]) instead of starting
    /// at `origin`.
    align_in: Option<Bounds>,
}

/// Copies placed in rows.
struct Block {
    placements: Vec<Placement>,
    holes: Vec<Area>,
    bounds: Option<Bounds>,
}

/// One row of a block: indices into the waiting copies, and their size together.
#[derive(Default)]
struct Row {
    members: Vec<usize>,
    length_mm: f64,
    width_mm: f64,
}

impl LayoutRequest<'_> {
    /// Places every copy of every product.
    pub(super) fn lay_out(&self) -> Layout {
        let mut layout = Layout::default();
        for placement in self.placements {
            // Validation guarantees the product exists.
            let Some(hole) = self.hole_for(&placement.product_id) else {
                continue;
            };
            layout.holes.push(hole.placed(placement));
            layout.placements.push(placement.clone());
        }
        let waiting = self.waiting();
        if waiting.is_empty() {
            return layout;
        }
        let placed = layout.bounds();
        let blocks = self
            .shelves((placed, &waiting))
            .into_iter()
            .map(|shelf| self.pack((&waiting, shelf)));
        let Some(block) = self.best_block((placed, blocks.collect())) else {
            return layout;
        };
        layout.generated_bounds = block.bounds;
        layout.placements.extend(block.placements);
        layout.holes.extend(block.holes);
        layout
    }

    fn hole_for(&self, product_id: &str) -> Option<&ProductHole> {
        let index = self
            .products
            .iter()
            .position(|product| product.id == product_id)?;
        self.holes.get(index)
    }

    /// Copies the design didn't place, product by product.
    fn waiting(&self) -> Vec<Waiting<'_>> {
        self.products
            .iter()
            .zip(self.holes)
            .flat_map(|(product, hole)| {
                let placed = self
                    .placements
                    .iter()
                    .filter(|placement| placement.product_id == product.id)
                    .count();
                let left = (product.quantity as usize).saturating_sub(placed);
                (0..left).map(move |_| Waiting {
                    product_id: &product.id,
                    hole,
                })
            })
            .collect()
    }

    /// Where the rows could go: centred in an empty space, or to the right of or below
    /// the products the design already placed.
    fn shelves(&self, (placed, waiting): (Option<Bounds>, &[Waiting])) -> Vec<Shelf> {
        let gap = self.gap_mm;
        let free_limit = self.free_row_limit(waiting);
        match (placed, self.space) {
            (None, Some(space)) => vec![Shelf {
                origin: space.min,
                row_limit_mm: space.length_mm(),
                align_in: Some(space),
            }],
            (None, None) => vec![Shelf {
                origin: Point::ORIGIN,
                row_limit_mm: free_limit,
                align_in: None,
            }],
            (Some(placed), Some(space)) => {
                let right = Point::new(placed.max.x + gap, space.min.y);
                vec![
                    Shelf {
                        origin: right,
                        row_limit_mm: space.max.x - right.x,
                        align_in: None,
                    },
                    Shelf {
                        origin: Point::new(space.min.x, placed.max.y + gap),
                        row_limit_mm: space.length_mm(),
                        align_in: None,
                    },
                ]
            }
            (Some(placed), None) => vec![
                Shelf {
                    origin: Point::new(placed.max.x + gap, placed.min.y),
                    row_limit_mm: free_limit,
                    align_in: None,
                },
                Shelf {
                    origin: Point::new(placed.min.x, placed.max.y + gap),
                    row_limit_mm: free_limit.max(placed.length_mm()),
                    align_in: None,
                },
            ],
        }
    }

    /// In a fixed space, the first block that fits (or the last, to report how much room
    /// it needs). Otherwise the block that keeps the card smallest.
    fn best_block(&self, (placed, blocks): (Option<Bounds>, Vec<Block>)) -> Option<Block> {
        if let Some(space) = self.space {
            let room = space.expanded(FIT_SLACK_MM);
            let fits = |block: &Block| block.bounds.is_none_or(|bounds| room.encloses(bounds));
            let first_fit = blocks.iter().position(fits);
            let chosen = first_fit.unwrap_or(blocks.len().saturating_sub(1));
            return blocks.into_iter().nth(chosen);
        }
        blocks.into_iter().min_by(|a, b| {
            let card_area = |block: &Block| {
                [placed, block.bounds]
                    .into_iter()
                    .flatten()
                    .reduce(Bounds::union)
                    .map_or(0.0, |bounds| bounds.length_mm() * bounds.width_mm())
            };
            card_area(a).total_cmp(&card_area(b))
        })
    }

    /// Lays the copies straight in rows, left to right and top to bottom.
    fn pack(&self, (waiting, shelf): (&[Waiting], Shelf)) -> Block {
        let lies: Vec<(Lie, Bounds)> = waiting
            .iter()
            .map(|copy| choose_lie((copy.hole, shelf.row_limit_mm)))
            .collect();
        let rows = self.rows((&lies, shelf.row_limit_mm));
        let (block_length, block_width) = self.block_size(&rows);
        let (share_x, share_y) = self.align.shares();
        let origin = match shelf.align_in {
            Some(space) => Point::new(
                space.min.x + (space.length_mm() - block_length) * share_x,
                space.min.y + (space.width_mm() - block_width) * share_y,
            ),
            None => shelf.origin,
        };

        let mut placements = Vec::with_capacity(waiting.len());
        let mut top = origin.y;
        for row in &rows {
            let mut left = origin.x + (block_length - row.length_mm) * share_x;
            for &index in &row.members {
                let (lie, footprint) = lies[index];
                let copy = &waiting[index];
                placements.push(Placement {
                    product_id: copy.product_id.to_owned(),
                    center_mm: Point::new(left, top) - footprint.min,
                    rotation_deg: copy.hole.rotation_deg(lie),
                });
                left += footprint.length_mm() + self.gap_mm;
            }
            top += row.width_mm + self.gap_mm;
        }
        let holes: Vec<Area> = placements
            .iter()
            .zip(waiting)
            .map(|(placement, copy)| copy.hole.placed(placement))
            .collect();
        Block {
            bounds: bounds_of(&holes),
            placements,
            holes,
        }
    }

    /// Row length for a card that grows. Tries the lengths at which rows change and
    /// keeps the one whose block has the shortest longest side (then the smallest area),
    /// so the card stays close to square.
    fn free_row_limit(&self, waiting: &[Waiting]) -> f64 {
        let lies: Vec<(Lie, Bounds)> = waiting
            .iter()
            .map(|copy| choose_lie((copy.hole, f64::INFINITY)))
            .collect();
        let scored = self.row_length_tries(&lies).into_iter().map(|limit| {
            let (length, width) = self.block_size(&self.rows((&lies, limit)));
            (limit, length.max(width), length * width)
        });
        scored
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)))
            .map_or(0.0, |(limit, ..)| limit)
    }

    /// Row lengths worth trying: the length of every run of consecutive copies, since
    /// rows only change where a run just fits. Repeats are dropped, and a long list is
    /// thinned evenly to [`MAX_ROW_LENGTH_TRIES`].
    fn row_length_tries(&self, lies: &[(Lie, Bounds)]) -> Vec<f64> {
        let lengths: Vec<f64> = lies.iter().map(|(_, bounds)| bounds.length_mm()).collect();
        let mut runs = Vec::new();
        for start in 0..lengths.len() {
            if runs.len() > MAX_ROW_LENGTHS_COLLECTED {
                break;
            }
            let mut run = -self.gap_mm;
            for length in &lengths[start..] {
                run += self.gap_mm + length;
                runs.push(run);
            }
        }
        runs.sort_by(f64::total_cmp);
        runs.dedup_by(|later, kept| *later - *kept < SAME_ROW_LENGTH_MM);
        let step = runs.len().div_ceil(MAX_ROW_LENGTH_TRIES).max(1);
        runs.into_iter().step_by(step).collect()
    }

    /// Length and width of a block of rows, with board between the rows.
    fn block_size(&self, rows: &[Row]) -> (f64, f64) {
        let length = rows.iter().map(|row| row.length_mm).fold(0.0, f64::max);
        let width = rows.iter().map(|row| row.width_mm).sum::<f64>()
            + self.gap_mm * rows.len().saturating_sub(1) as f64;
        (length, width)
    }

    /// Breaks the copies into rows no longer than `row_limit_mm`. A copy longer than the
    /// limit gets a row of its own; the fit check then reports it.
    fn rows(&self, (lies, row_limit_mm): (&[(Lie, Bounds)], f64)) -> Vec<Row> {
        let mut rows: Vec<Row> = Vec::new();
        let mut row = Row::default();
        for (index, (_, footprint)) in lies.iter().enumerate() {
            let length = footprint.length_mm();
            let longer = match row.members.is_empty() {
                true => length,
                false => row.length_mm + self.gap_mm + length,
            };
            if !row.members.is_empty() && longer > row_limit_mm + FIT_SLACK_MM {
                rows.push(std::mem::take(&mut row));
                row.length_mm = length;
            } else {
                row.length_mm = longer;
            }
            row.members.push(index);
            row.width_mm = row.width_mm.max(footprint.width_mm());
        }
        if !row.members.is_empty() {
            rows.push(row);
        }
        rows
    }
}

/// Lengthwise, unless only crosswise fits the row.
fn choose_lie((hole, row_limit_mm): (&ProductHole, f64)) -> (Lie, Bounds) {
    let footprint = |lie| {
        (
            lie,
            hole.footprint(lie).unwrap_or(Bounds::from_size((0.0, 0.0))),
        )
    };
    let lengthwise = footprint(Lie::Lengthwise);
    let crosswise = footprint(Lie::Crosswise);
    let fits = |(_, bounds): (Lie, Bounds)| bounds.length_mm() <= row_limit_mm + FIT_SLACK_MM;
    match !fits(lengthwise) && fits(crosswise) {
        true => crosswise,
        false => lengthwise,
    }
}

fn bounds_of(holes: &[Area]) -> Option<Bounds> {
    holes.iter().filter_map(Area::bounds).reduce(Bounds::union)
}
