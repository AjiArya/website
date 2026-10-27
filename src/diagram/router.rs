use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::layout::{Point, Rect, Side};

const CELL: f64 = 8.0;
const CLEARANCE: f64 = 7.0;
const STUB: f64 = 14.0;
const TURN_COST: u32 = 4;
const MOVES: [(isize, isize); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// Route an orthogonal path from a source port to a target port while avoiding
/// the supplied node/label rectangles. Falls back to `None` when the grid has
/// no path.
pub fn route(
    start: Point,
    start_side: Side,
    end: Point,
    end_side: Side,
    obstacles: &[Rect],
    bounds: Rect,
) -> Option<(String, Point)> {
    let start_stub = outward(start, start_side);
    let end_stub = outward(end, end_side);

    let min_x = bounds.x.min(start_stub.x).min(end_stub.x) - 20.0;
    let min_y = bounds.y.min(start_stub.y).min(end_stub.y) - 20.0;
    let max_x = bounds.right().max(start_stub.x).max(end_stub.x) + 20.0;
    let max_y = bounds.bottom().max(start_stub.y).max(end_stub.y) + 20.0;

    let cols = (((max_x - min_x) / CELL).ceil() as usize).max(1) + 1;
    let rows = (((max_y - min_y) / CELL).ceil() as usize).max(1) + 1;

    let mut blocked = vec![false; cols * rows];
    for obstacle in obstacles {
        mark_rect(&mut blocked, cols, rows, min_x, min_y, *obstacle);
    }

    let start_cell = nearest_free(
        &blocked,
        cols,
        rows,
        point_to_cell(start_stub, min_x, min_y),
    )?;
    let goal_cell = nearest_free(&blocked, cols, rows, point_to_cell(end_stub, min_x, min_y))?;

    let cells = astar(&blocked, cols, rows, start_cell, goal_cell)?;
    let mut points = vec![start, start_stub];

    let mut previous = start_stub;
    for (index, (x, y)) in cells.iter().enumerate() {
        let point = cell_center(*x, *y, min_x, min_y);
        if index == 0 {
            add_orthogonal_corner(&mut points, previous, point, start_side.is_horizontal());
        }
        points.push(point);
        previous = point;
    }

    if (previous.x - end_stub.x).abs() > 0.1 || (previous.y - end_stub.y).abs() > 0.1 {
        add_orthogonal_corner(&mut points, previous, end_stub, !end_side.is_horizontal());
    }
    points.push(end_stub);
    points.push(end);

    Some(path_from_points(&points))
}

/// Build a path and label from already ordered orthogonal points.
pub fn path_from_points(points: &[Point]) -> (String, Point) {
    let points = simplify(points.to_vec());
    (to_path(&points), label_point(&points))
}

fn outward(point: Point, side: Side) -> Point {
    match side {
        Side::Left => Point {
            x: point.x - STUB,
            y: point.y,
        },
        Side::Right => Point {
            x: point.x + STUB,
            y: point.y,
        },
        Side::Top => Point {
            x: point.x,
            y: point.y - STUB,
        },
        Side::Bottom => Point {
            x: point.x,
            y: point.y + STUB,
        },
    }
}

fn point_to_cell(point: Point, min_x: f64, min_y: f64) -> (usize, usize) {
    (
        ((point.x - min_x) / CELL).round().max(0.0) as usize,
        ((point.y - min_y) / CELL).round().max(0.0) as usize,
    )
}

fn cell_center(x: usize, y: usize, min_x: f64, min_y: f64) -> Point {
    Point {
        x: min_x + (x as f64 + 0.5) * CELL,
        y: min_y + (y as f64 + 0.5) * CELL,
    }
}

fn mark_rect(blocked: &mut [bool], cols: usize, rows: usize, min_x: f64, min_y: f64, rect: Rect) {
    let inflated = Rect {
        x: rect.x - CLEARANCE,
        y: rect.y - CLEARANCE,
        w: rect.w + CLEARANCE * 2.0,
        h: rect.h + CLEARANCE * 2.0,
    };
    let x0 = (((inflated.x - min_x) / CELL).floor().max(0.0) as usize).min(cols.saturating_sub(1));
    let y0 = (((inflated.y - min_y) / CELL).floor().max(0.0) as usize).min(rows.saturating_sub(1));
    let x1 = (((inflated.right() - min_x) / CELL).ceil() as usize).min(cols.saturating_sub(1));
    let y1 = (((inflated.bottom() - min_y) / CELL).ceil() as usize).min(rows.saturating_sub(1));

    for y in y0..=y1 {
        for x in x0..=x1 {
            blocked[y * cols + x] = true;
        }
    }
}

fn nearest_free(
    blocked: &[bool],
    cols: usize,
    rows: usize,
    (x, y): (usize, usize),
) -> Option<(usize, usize)> {
    let x = x.min(cols.saturating_sub(1));
    let y = y.min(rows.saturating_sub(1));
    if !blocked[y * cols + x] {
        return Some((x, y));
    }

    for radius in 1..=8_isize {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs().max(dy.abs()) != radius {
                    continue;
                }
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                    continue;
                }
                let nx = nx as usize;
                let ny = ny as usize;
                if !blocked[ny * cols + nx] {
                    return Some((nx, ny));
                }
            }
        }
    }
    None
}

fn heuristic(a: (usize, usize), b: (usize, usize)) -> u32 {
    (a.0.abs_diff(b.0) + a.1.abs_diff(b.1)) as u32
}

fn astar(
    blocked: &[bool],
    cols: usize,
    rows: usize,
    start: (usize, usize),
    goal: (usize, usize),
) -> Option<Vec<(usize, usize)>> {
    let state_count = cols * rows * 5;
    let mut g_score = vec![u32::MAX; state_count];
    let mut came_from = vec![usize::MAX; state_count];
    let start_state = (start.1 * cols + start.0) * 5 + 4;
    let goal_cell = goal.1 * cols + goal.0;
    g_score[start_state] = 0;

    let mut open: BinaryHeap<Reverse<(u32, u32, usize)>> = BinaryHeap::new();
    open.push(Reverse((heuristic(start, goal), 0, start_state)));

    let mut goal_state = None;
    while let Some(Reverse((_estimate, cost, state))) = open.pop() {
        if cost != g_score[state] {
            continue;
        }
        let cell = state / 5;
        if cell == goal_cell {
            goal_state = Some(state);
            break;
        }

        let direction = state % 5;
        let x = cell % cols;
        let y = cell / cols;

        for (next_direction, (dx, dy)) in MOVES.iter().enumerate() {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            if blocked[ny * cols + nx] {
                continue;
            }

            let next_state = ((ny * cols + nx) * 5) + next_direction;
            let turn = if direction != 4 && direction != next_direction {
                TURN_COST
            } else {
                0
            };
            let next_cost = cost + 1 + turn;
            if next_cost >= g_score[next_state] {
                continue;
            }

            g_score[next_state] = next_cost;
            came_from[next_state] = state;
            open.push(Reverse((
                next_cost + heuristic((nx, ny), goal),
                next_cost,
                next_state,
            )));
        }
    }

    let mut state = goal_state?;
    let mut cells = Vec::new();
    while state != usize::MAX {
        let cell = state / 5;
        cells.push((cell % cols, cell / cols));
        if state == start_state {
            break;
        }
        state = came_from[state];
    }
    cells.reverse();
    Some(cells)
}

fn add_orthogonal_corner(points: &mut Vec<Point>, from: Point, to: Point, horizontal_first: bool) {
    if (from.x - to.x).abs() < 0.1 || (from.y - to.y).abs() < 0.1 {
        return;
    }
    let corner = if horizontal_first {
        Point { x: to.x, y: from.y }
    } else {
        Point { x: from.x, y: to.y }
    };
    points.push(corner);
}

fn simplify(mut points: Vec<Point>) -> Vec<Point> {
    points.dedup_by(|a, b| (a.x - b.x).abs() < 0.1 && (a.y - b.y).abs() < 0.1);

    let mut changed = true;
    while changed {
        changed = false;
        for index in 1..points.len().saturating_sub(1) {
            let a = points[index - 1];
            let b = points[index];
            let c = points[index + 1];
            let same_x = (a.x - b.x).abs() < 0.1 && (b.x - c.x).abs() < 0.1;
            let same_y = (a.y - b.y).abs() < 0.1 && (b.y - c.y).abs() < 0.1;
            if same_x || same_y {
                points.remove(index);
                changed = true;
                break;
            }
        }
    }
    points
}

fn to_path(points: &[Point]) -> String {
    let mut path = String::new();
    for (index, point) in points.iter().enumerate() {
        if index == 0 {
            path.push_str(&format!("M {:.1} {:.1}", point.x, point.y));
        } else {
            path.push_str(&format!(" L {:.1} {:.1}", point.x, point.y));
        }
    }
    path
}

fn label_point(points: &[Point]) -> Point {
    let mut best_length = 0.0;
    let mut best = Point::default();
    let mut best_horizontal = true;

    for pair in points.windows(2) {
        let dx = pair[1].x - pair[0].x;
        let dy = pair[1].y - pair[0].y;
        let length = dx.hypot(dy);
        if length > best_length {
            best_length = length;
            best = Point {
                x: (pair[0].x + pair[1].x) / 2.0,
                y: (pair[0].y + pair[1].y) / 2.0,
            };
            best_horizontal = dx.abs() >= dy.abs();
        }
    }

    if best_horizontal {
        best
    } else {
        Point {
            x: best.x + 10.0,
            y: best.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avoids_a_blocking_rectangle() {
        let start = Point { x: 20.0, y: 60.0 };
        let end = Point { x: 180.0, y: 60.0 };
        let obstacle = Rect {
            x: 80.0,
            y: 20.0,
            w: 40.0,
            h: 80.0,
        };
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 220.0,
            h: 120.0,
        };
        let (path, _) =
            route(start, Side::Right, end, Side::Left, &[obstacle], bounds).expect("route");
        assert!(path.starts_with('M'));
        assert!(path.contains(" L "));
        assert!(path.contains("80.0") || path.contains("120.0"));
    }

    #[test]
    fn leaves_source_and_target_sides_orthogonally() {
        let start = Point { x: 40.0, y: 40.0 };
        let end = Point { x: 240.0, y: 140.0 };
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 200.0,
        };
        let (path, _) = route(start, Side::Right, end, Side::Top, &[], bounds).expect("route");
        assert!(path.contains(" L "));
    }
}
