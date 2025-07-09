// Geometric utilities for computer vision and UI positioning
// Custom implementations without external geometry crates

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn midpoint(&self, other: &Point) -> Point {
        Point::new((self.x + other.x) / 2.0, (self.y + other.y) / 2.0)
    }

    pub fn translate(&self, dx: f64, dy: f64) -> Point {
        Point::new(self.x + dx, self.y + dy)
    }

    pub fn rotate_around(&self, center: &Point, angle_radians: f64) -> Point {
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();
        
        let dx = self.x - center.x;
        let dy = self.y - center.y;
        
        Point::new(
            center.x + dx * cos_a - dy * sin_a,
            center.y + dx * sin_a + dy * cos_a,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rectangle {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_points(p1: Point, p2: Point) -> Self {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let width = (p2.x - p1.x).abs();
        let height = (p2.y - p1.y).abs();
        Self::new(x, y, width, height)
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn top_right(&self) -> Point {
        Point::new(self.x + self.width, self.y)
    }

    pub fn bottom_left(&self) -> Point {
        Point::new(self.x, self.y + self.height)
    }

    pub fn bottom_right(&self) -> Point {
        Point::new(self.x + self.width, self.y + self.height)
    }

    pub fn contains_point(&self, point: &Point) -> bool {
        point.x >= self.x && point.x <= self.x + self.width &&
        point.y >= self.y && point.y <= self.y + self.height
    }

    pub fn intersects(&self, other: &Rectangle) -> bool {
        !(self.x + self.width < other.x ||
          other.x + other.width < self.x ||
          self.y + self.height < other.y ||
          other.y + other.height < self.y)
    }

    pub fn intersection(&self, other: &Rectangle) -> Option<Rectangle> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);

        Some(Rectangle::new(x, y, right - x, bottom - y))
    }

    pub fn union(&self, other: &Rectangle) -> Rectangle {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);

        Rectangle::new(x, y, right - x, bottom - y)
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn perimeter(&self) -> f64 {
        2.0 * (self.width + self.height)
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.width / self.height
    }

    pub fn scale(&self, factor: f64) -> Rectangle {
        let center = self.center();
        let new_width = self.width * factor;
        let new_height = self.height * factor;
        Rectangle::new(
            center.x - new_width / 2.0,
            center.y - new_height / 2.0,
            new_width,
            new_height,
        )
    }

    pub fn expand(&self, margin: f64) -> Rectangle {
        Rectangle::new(
            self.x - margin,
            self.y - margin,
            self.width + 2.0 * margin,
            self.height + 2.0 * margin,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

impl Circle {
    pub fn new(center: Point, radius: f64) -> Self {
        Self { center, radius }
    }

    pub fn contains_point(&self, point: &Point) -> bool {
        self.center.distance_to(point) <= self.radius
    }

    pub fn intersects_rectangle(&self, rect: &Rectangle) -> bool {
        // Find the closest point on the rectangle to the circle center
        let closest_x = self.center.x.max(rect.x).min(rect.x + rect.width);
        let closest_y = self.center.y.max(rect.y).min(rect.y + rect.height);
        let closest = Point::new(closest_x, closest_y);

        self.center.distance_to(&closest) <= self.radius
    }

    pub fn area(&self) -> f64 {
        PI * self.radius * self.radius
    }

    pub fn circumference(&self) -> f64 {
        2.0 * PI * self.radius
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub points: Vec<Point>,
}

impl Polygon {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    pub fn bounding_rectangle(&self) -> Option<Rectangle> {
        if self.points.is_empty() {
            return None;
        }

        let mut min_x = self.points[0].x;
        let mut max_x = self.points[0].x;
        let mut min_y = self.points[0].y;
        let mut max_y = self.points[0].y;

        for point in &self.points[1..] {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        Some(Rectangle::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }

    pub fn contains_point(&self, point: &Point) -> bool {
        // Ray casting algorithm
        let mut inside = false;
        let mut j = self.points.len() - 1;

        for i in 0..self.points.len() {
            let pi = &self.points[i];
            let pj = &self.points[j];

            if ((pi.y > point.y) != (pj.y > point.y)) &&
               (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x) {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    pub fn area(&self) -> f64 {
        if self.points.len() < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        let mut j = self.points.len() - 1;

        for i in 0..self.points.len() {
            area += (self.points[j].x + self.points[i].x) * (self.points[j].y - self.points[i].y);
            j = i;
        }

        area.abs() / 2.0
    }

    pub fn centroid(&self) -> Option<Point> {
        if self.points.is_empty() {
            return None;
        }

        let sum_x: f64 = self.points.iter().map(|p| p.x).sum();
        let sum_y: f64 = self.points.iter().map(|p| p.y).sum();

        Some(Point::new(
            sum_x / self.points.len() as f64,
            sum_y / self.points.len() as f64,
        ))
    }
}

// Utility functions for geometric calculations
pub fn angle_between_points(p1: &Point, p2: &Point) -> f64 {
    (p2.y - p1.y).atan2(p2.x - p1.x)
}

pub fn normalize_angle(angle: f64) -> f64 {
    let mut result = angle % (2.0 * PI);
    if result < 0.0 {
        result += 2.0 * PI;
    }
    result
}

pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

pub fn radians_to_degrees(radians: f64) -> f64 {
    radians * 180.0 / PI
}

pub fn linear_interpolate(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t.clamp(0.0, 1.0)
}

pub fn point_interpolate(start: &Point, end: &Point, t: f64) -> Point {
    Point::new(
        linear_interpolate(start.x, end.x, t),
        linear_interpolate(start.y, end.y, t),
    )
}

// Grid-based spatial partitioning for efficient collision detection
pub struct SpatialGrid {
    cell_size: f64,
    objects: std::collections::HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            objects: std::collections::HashMap::new(),
        }
    }

    fn get_cell(&self, point: &Point) -> (i32, i32) {
        (
            (point.x / self.cell_size).floor() as i32,
            (point.y / self.cell_size).floor() as i32,
        )
    }

    pub fn insert(&mut self, id: usize, bounds: &Rectangle) {
        let min_cell = self.get_cell(&bounds.top_left());
        let max_cell = self.get_cell(&bounds.bottom_right());

        for x in min_cell.0..=max_cell.0 {
            for y in min_cell.1..=max_cell.1 {
                self.objects.entry((x, y)).or_insert_with(Vec::new).push(id);
            }
        }
    }

    pub fn query(&self, bounds: &Rectangle) -> Vec<usize> {
        let min_cell = self.get_cell(&bounds.top_left());
        let max_cell = self.get_cell(&bounds.bottom_right());
        let mut result = Vec::new();

        for x in min_cell.0..=max_cell.0 {
            for y in min_cell.1..=max_cell.1 {
                if let Some(objects) = self.objects.get(&(x, y)) {
                    for &id in objects {
                        if !result.contains(&id) {
                            result.push(id);
                        }
                    }
                }
            }
        }

        result
    }

    pub fn clear(&mut self) {
        self.objects.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_operations() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);

        assert_eq!(p1.distance_to(&p2), 5.0);
        assert_eq!(p1.midpoint(&p2), Point::new(1.5, 2.0));
    }

    #[test]
    fn test_rectangle_operations() {
        let rect1 = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let rect2 = Rectangle::new(5.0, 5.0, 10.0, 10.0);

        assert!(rect1.intersects(&rect2));
        assert_eq!(rect1.area(), 100.0);
        assert_eq!(rect1.center(), Point::new(5.0, 5.0));

        let intersection = rect1.intersection(&rect2).unwrap();
        assert_eq!(intersection, Rectangle::new(5.0, 5.0, 5.0, 5.0));
    }

    #[test]
    fn test_circle_operations() {
        let circle = Circle::new(Point::new(0.0, 0.0), 5.0);
        let point_inside = Point::new(3.0, 3.0);
        let point_outside = Point::new(6.0, 6.0);

        assert!(circle.contains_point(&point_inside));
        assert!(!circle.contains_point(&point_outside));
    }

    #[test]
    fn test_polygon_operations() {
        let triangle = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(5.0, 10.0),
        ]);

        let inside_point = Point::new(5.0, 3.0);
        let outside_point = Point::new(15.0, 5.0);

        assert!(triangle.contains_point(&inside_point));
        assert!(!triangle.contains_point(&outside_point));
        assert_eq!(triangle.area(), 50.0);
    }

    #[test]
    fn test_spatial_grid() {
        let mut grid = SpatialGrid::new(10.0);
        let rect = Rectangle::new(5.0, 5.0, 10.0, 10.0);

        grid.insert(1, &rect);
        let results = grid.query(&Rectangle::new(0.0, 0.0, 20.0, 20.0));

        assert!(results.contains(&1));
    }
}