use euclid::{Point2D, Rect};
use internal_types::{ClipRectResult, WorkVertex};
use types::{BoxShadowClipMode, ColorF};

fn is_inside(a: &Point2D<f32>, b: &Point2D<f32>, c: &WorkVertex) -> bool {
    (a.x - c.x) * (b.y - c.y) > (a.y - c.y) * (b.x - c.x)
}

fn intersection(a: &Point2D<f32>, b: &Point2D<f32>, p: &WorkVertex, q: &WorkVertex) -> WorkVertex {
    let denominator = (a.x - b.x) * (p.y - q.y) - (a.y - b.y) * (p.x - q.x);
    let x = ((a.x*b.y-b.x*a.y) * (p.x-q.x) - (a.x-b.x) * (p.x*q.y-p.y*q.x)) / denominator;
    let y = ((a.x*b.y-b.x*a.y) * (p.y-q.y) - (a.y-b.y) * (p.x*q.y-p.y*q.x)) / denominator;

    let d1 = ((p.x - x) * (p.x - x) + (p.y - y) * (p.y - y)).sqrt();
    let d2 = ((p.x - q.x) * (p.x - q.x) + (p.y - q.y) * (p.y - q.y)).sqrt();
    let ratio = d1 / d2;

    let u = p.u + ratio * (q.u - p.u);
    let v = p.v + ratio * (q.v - p.v);

    let mu = p.mu + ratio * (q.mu - p.mu);
    let mv = p.mv + ratio * (q.mv - p.mv);

    let r = p.r + ratio * (q.r - p.r);
    let g = p.g + ratio * (q.g - p.g);
    let b = p.b + ratio * (q.b - p.b);
    let a = p.a + ratio * (q.a - p.a);

    let color = ColorF::new(r, g, b, a);

    WorkVertex::new(x, y, &color, u, v, mu, mv)
}

pub fn clip_polygon(polygon: &Vec<WorkVertex>, clip_polygon: &Vec<Point2D<f32>>) -> Vec<WorkVertex> {
    let mut result = polygon.clone();

    let clip_len = clip_polygon.len();

    for i in 0..clip_len {
        let input = result.clone();
        let input_len = input.len();
        result.clear();

        let a = &clip_polygon[(i + clip_len-1) % clip_len];
        let b = &clip_polygon[i];

        for j in 0..input.len() {
            let p = &input[(j + input_len-1) % input_len];
            let q = &input[j];

            if is_inside(a, b, q) {
                if !is_inside(a, b, p) {
                    result.push(intersection(a, b, p, q));
                }
                result.push(q.clone());
            } else if is_inside(a, b, p) {
                result.push(intersection(a, b, p, q));
            }
        }
    }

    result
}

pub fn clip_rect_pos_uv(pos: &Rect<f32>, uv: &Rect<f32>, clip_rect: &Rect<f32>) -> Option<ClipRectResult> {
    pos.intersection(clip_rect).map(|clipped_rect| {
        let cx0 = clipped_rect.origin.x;
        let cy0 = clipped_rect.origin.y;
        let cx1 = cx0 + clipped_rect.size.width;
        let cy1 = cy0 + clipped_rect.size.height;

        let f0 = (cx0 - pos.origin.x) / pos.size.width;
        let f1 = (cy0 - pos.origin.y) / pos.size.height;
        let f2 = (cx1 - pos.origin.x) / pos.size.width;
        let f3 = (cy1 - pos.origin.y) / pos.size.height;

        ClipRectResult {
            x0: cx0,
            y0: cy0,
            x1: cx1,
            y1: cy1,
            u0: uv.origin.x + f0 * uv.size.width,
            v0: uv.origin.y + f1 * uv.size.height,
            u1: uv.origin.x + f2 * uv.size.width,
            v1: uv.origin.y + f3 * uv.size.height,
        }
    })
}

pub fn clip_out_rect_pos_uv(pos: &Rect<f32>, uv: &Rect<f32>, clip_rect: &Rect<f32>)
                            -> Vec<ClipRectResult> {
    let clip_rect = match pos.intersection(clip_rect) {
        Some(clip_rect) => clip_rect,
        None => return vec![ClipRectResult::from_rects(pos, uv)],
    };

    // FIXME(pcwalton): Clip the u and v too.
    let mut result = vec![];
    push(&mut result, uv, &pos.origin, &Point2D::new(pos.max_x(), clip_rect.origin.y));
    push(&mut result,
         uv,
         &Point2D::new(pos.origin.x, clip_rect.origin.y),
         &clip_rect.bottom_left());
    push(&mut result, uv, &clip_rect.top_right(), &Point2D::new(pos.max_x(), clip_rect.max_y()));
    push(&mut result, uv, &Point2D::new(pos.origin.x, clip_rect.max_y()), &pos.bottom_right());
    return result;

    fn push(result: &mut Vec<ClipRectResult>,
            uv: &Rect<f32>,
            top_left: &Point2D<f32>,
            bottom_right: &Point2D<f32>) {
        if top_left.x >= bottom_right.x || top_left.y >= bottom_right.y {
            return
        }
        result.push(ClipRectResult {
            x0: top_left.x,
            y0: top_left.y,
            x1: bottom_right.x,
            y1: bottom_right.y,
            u0: uv.origin.x,
            v0: uv.origin.y,
            u1: uv.max_x(),
            v1: uv.max_y(),
        })
    }
}

pub fn clip_rect_with_mode_pos_uv(pos: &Rect<f32>,
                                  uv: &Rect<f32>,
                                  clip_rect: &Rect<f32>,
                                  clip_mode: BoxShadowClipMode)
                                  -> Vec<ClipRectResult> {
    match clip_mode {
        BoxShadowClipMode::None => vec![ClipRectResult::from_rects(pos, uv)],
        BoxShadowClipMode::Inset => {
            match clip_rect_pos_uv(pos, uv, clip_rect) {
                Some(clip_result) => vec![clip_result],
                None => vec![],
            }
        }
        BoxShadowClipMode::Outset => {
            clip_out_rect_pos_uv(pos, uv, clip_rect)
        }
    }
}

