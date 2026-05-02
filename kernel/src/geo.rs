use crate::math::*;
use alloc::{collections::btree_set::BTreeSet, vec::Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Points<T: Arithmetic> {
    pub points: Vec<Vec2<T>>,
}

impl<T: Arithmetic + core::cmp::Ord> Points<T> {
    pub fn new(points: Vec<Vec2<T>>) -> Self {
        Self { points }
    }

    pub fn as_usize(&self) -> Points<usize> {
        Points::new(self.points.iter().map(|p| p.as_usize()).collect())
    }

    pub fn contain(&self, point: Vec2<T>) -> bool {
        self.points.iter().any(|p| p == &point)
    }

    pub fn intersection(&self, points: Points<T>) -> Points<T> {
        Points::new(
            self.points
                .iter()
                .cloned()
                .filter(|p| points.contain(*p))
                .collect(),
        )
    }
    pub fn difference(&self, other: &Points<T>) -> Points<T> {
        let other_set: BTreeSet<Vec2<T>> = other.points.iter().cloned().collect();
        let mut out = Vec::with_capacity(self.points.len());
        for &p in &self.points {
            if !other_set.contains(&p) {
                out.push(p);
            }
        }
        Points::new(out)
    }

    pub fn union(&self, points: Points<T>) -> Points<T> {
        Points::new(
            self.points
                .iter()
                .cloned()
                .chain(self.difference(&points).points.iter().cloned())
                .collect(),
        )
    }

    pub fn push(&mut self, point: Vec2<T>) {
        self.points.push(point);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Rect<T: Arithmetic> {
    pub p0: Vec2<T>,
    pub p1: Vec2<T>,
    pub fill: bool,
}

impl<T: Arithmetic> Rect<T> {
    pub fn new(p0: Vec2<T>, p1: Vec2<T>, fill: bool) -> Self {
        Self { p0, p1, fill }
    }

    pub fn as_usize(self) -> Rect<usize> {
        Rect {
            p0: self.p0.as_usize(),
            p1: self.p1.as_usize(),
            fill: self.fill,
        }
    }

    pub fn top_left(&self) -> Vec2<T> {
        self.p0
    }

    pub fn bottom_right(&self) -> Vec2<T> {
        self.p1
    }

    pub fn center(&self) -> Vec2<T> {
        (self.p0 + self.p1) / T::new(2.0)
    }

    pub fn size(&self) -> Vec2<T> {
        self.p1 - self.p0
    }

    pub fn area(&self) -> T {
        self.size().x * self.size().y
    }

    pub fn into<K: From<T> + Arithmetic>(self) -> Rect<K> {
        Rect {
            p0: self.p0.into(),
            p1: self.p1.into(),
            fill: self.fill,
        }
    }
}

pub fn rect<T: Arithmetic>(p0: Vec2<T>, p1: Vec2<T>) -> Rect<T> {
    Rect::new(p0, p1, true)
}
pub fn rect_nofill<T: Arithmetic>(p0: Vec2<T>, p1: Vec2<T>) -> Rect<T> {
    Rect::new(p0, p1, false)
}


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Line<T: Arithmetic> {
    pub start: Vec2<T>,
    pub end: Vec2<T>,
}

impl<T: Arithmetic> Line<T> {
    pub fn new(start: Vec2<T>, end: Vec2<T>) -> Self {
        Self { start, end }
    }

    pub fn length(&self) -> T {
        (self.end - self.start).magnitude()
    }

    pub fn into<K: From<T> + Arithmetic>(self) -> Line<K> {
        Line {
            start: self.start.into(),
            end: self.end.into(),
        }
    }
}

pub fn line<T: Arithmetic>(start: Vec2<T>, end: Vec2<T>) -> Line<T> {
    Line::new(start, end)
}

pub struct Circle<T: Arithmetic> {
    pub center: Vec2<T>,
    pub radius: T,
    pub fill: bool,
}

impl<T: Arithmetic> Circle<T> {
    pub fn new(center: Vec2<T>, radius: T, fill: bool) -> Self {
        Self {
            center,
            radius,
            fill,
        }
    }

    pub fn rect(&self) -> Rect<T> {
        rect(
            vec2(self.center.x - self.radius, self.center.y - self.radius),
            vec2(self.center.x + self.radius, self.center.y + self.radius),
        )
    }

    pub fn area(&self) -> T {
        T::pi() * self.radius * self.radius
    }

    pub fn circumference(&self) -> T {
        T::pi() * self.radius * T::new(2.0)
    }

    pub fn into<K: From<T> + Arithmetic>(self) -> Circle<K> {
        Circle {
            center: self.center.into(),
            radius: self.radius.into(),
            fill: self.fill,
        }
    }
}

pub fn circle<T: Arithmetic>(center: Vec2<T>, radius: T, fill: bool) -> Circle<T> {
    Circle::new(center, radius, fill)
}

pub trait Object<T: Arithmetic> {
    fn points(&self) -> Points<T>;
    fn translate(&self, delta: Vec2<T>) -> Self;

    fn rect(&self) -> Option<Rect<T>>;
}

impl<T: Arithmetic + Ord> Object<T> for Points<T> {
    fn points(&self) -> Points<T> {
        self.clone()
    }

    fn translate(&self, delta: Vec2<T>) -> Self {
        Points::new(self.points.iter().map(|p| *p + delta).collect())
    }

    fn rect(&self) -> Option<Rect<T>> {
        None
    }
}

impl<T: Arithmetic + core::cmp::Ord> Object<T> for Rect<T> {
    fn points(&self) -> Points<T> {
        let mut points = Points::new(Vec::new());
        let [x1, y1] = self.top_left().to_slice();
        let [x2, y2] = self.bottom_right().to_slice();
        if self.fill {
            for y in T::range(y1, y2) {
                for x in T::range(x1, x2) {
                    points.push(Vec2::new(x, y));
                }
            }
        } else {
            for x in T::range(x1, x2) {
                points.push(Vec2::new(x, y1));
                points.push(Vec2::new(x, y2));
            }
            for y in T::range(y1, y2) {
                points.push(Vec2::new(x1, y));
                points.push(Vec2::new(x2, y));
            }
        }

        points
    }

    fn translate(&self, delta: Vec2<T>) -> Self {
        Rect::new(self.top_left() + delta, self.bottom_right() + delta, self.fill)
    }

    fn rect(&self) -> Option<Rect<T>> {
        Some(*self)
    }
}

impl<T: Arithmetic + Ord> Object<T> for Line<T> {
    fn points(&self) -> Points<T> {
        let mut points = Points::new(Vec::new());
        let [x1, y1] = self.start.to_slice();
        let [x2, y2] = self.end.to_slice();


        let dx = x2 - x1;
        let dy = y2 - y1;

        let mut x = x1;
        let mut y = y1;

        let mut error = 0;

        while x != x2 {
            points.push(Vec2::new(x, y));
            error += dy.as_usize();

            if error > dx.as_usize() {
                y = y + T::new(1.0);
                error -= dx.as_usize();
            }

            x = x + T::new(1.0);
        }

        points
    }

    fn translate(&self, delta: Vec2<T>) -> Self {
        Line::new(self.start + delta, self.end + delta)
    }

    fn rect(&self) -> Option<Rect<T>> {
        let min_x = self.start.x.min(self.end.x);
        let max_x = self.start.x.max(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_y = self.start.y.max(self.end.y);

        Some(Rect::new(
            Vec2::new(min_x, min_y),
            Vec2::new(max_x, max_y),
            false,
        ))
    }
}

impl<T: Arithmetic + core::cmp::Ord> Object<T> for Circle<T> {
    fn points(&self) -> Points<T> {
        let mut points = Points::new(Vec::new());
        let mut x = T::new(0.0);
        let mut y = self.radius;
        let mut d = T::new(3.0) - T::new(2.0) * self.radius;
        let cx = self.center.x;
        let cy = self.center.y;
        if self.fill {
            let mut draw_horizontal = |start_x: T, end_x: T, y: T| {
                let x1 = start_x.max(T::new(0.0));
                let x2 = end_x.max(T::new(0.0));
                let py = y.max(T::new(0.0));
                let line = Line::new(Vec2::new(x1, py), Vec2::new(x2, py));
                points.points.extend(line.points().points);
            };

            while y >= x {
                draw_horizontal(cx - x, cx + x, cy + y);
                draw_horizontal(cx - x, cx + x, cy - y);
                draw_horizontal(cx - y, cx + y, cy + x);
                draw_horizontal(cx - y, cx + y, cy - x);

                x += T::new(1.0);
                if d > T::new(0.0) {
                    y -= T::new(1.0);
                    d = d + T::new(4.0) * (x - y) + T::new(10.0);
                } else {
                    d = d + T::new(4.0) * x + T::new(6.0);
                }
            }
        } else {
            let mut draw_point = |dx: T, dy: T| {
                let px = cx + dx;
                let py = cy + dy;
                points.push(Vec2::new(px, py));
            };

            while y >= x {
                draw_point(x, y);
                draw_point(x.neg(), y);
                draw_point(x, y.neg());
                draw_point(x.neg(), y.neg());
                draw_point(y, x);
                draw_point(y.neg(), x);
                draw_point(y, x.neg());
                draw_point(y.neg(), x.neg());

                x += T::new(1.0);
                if d > T::new(0.0) {
                    y -= T::new(1.0);
                    d = d + T::new(4.0) * (x - y) + T::new(10.0);
                } else {
                    d = d + T::new(4.0) * x + T::new(6.0);
                }
            }
        }
        points
    }

    fn translate(&self, delta: Vec2<T>) -> Self {
        Circle::new(self.center + delta, self.radius, self.fill)
    }

    fn rect(&self) -> Option<Rect<T>> {
        Some(Rect::new(
            vec2(self.center.x - self.radius, self.center.y - self.radius),
            vec2(self.center.x + self.radius, self.center.y + self.radius),
            self.fill,
        ))

    }
}
