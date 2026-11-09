//! Bounding-volume hierarchy over the finite shapes.

use super::{Aabb, Shape};
use crate::math::Vec3;

/// Node of the BVH: a leaf holds `count` objects from `first` in the
/// ordered index list, an inner node its two children.
#[derive(Clone, Debug)]
struct Node {
    bounds: Aabb,
    first: usize,
    count: usize,
    left: usize,
    right: usize,
}

/// Bounding-volume hierarchy over the finite shapes.
#[derive(Clone, Debug, Default)]
pub struct Bvh {
    nodes: Vec<Node>,
    order: Vec<usize>,
}

const LEAF_SIZE: usize = 4;

impl Bvh {
    /// Builds the tree over `shapes[i]` for the indices in `members`
    /// (deterministic: median split on the longest axis, ties by index).
    /// Members without finite bounds (infinite planes) are left out.
    pub fn build(shapes: &[Shape], members: Vec<usize>) -> Bvh {
        let boxes: Vec<Aabb> = shapes
            .iter()
            .map(|shape| shape.bounds().unwrap_or(Aabb::point(Vec3::default())))
            .collect();
        let order: Vec<usize> = members
            .into_iter()
            .filter(|&index| shapes.get(index).is_some_and(|s| s.bounds().is_some()))
            .collect();
        let mut bvh = Bvh {
            nodes: Vec::new(),
            order,
        };
        if !bvh.order.is_empty() {
            let count = bvh.order.len();
            bvh.split(&boxes, 0, count);
        }
        bvh
    }

    fn split(&mut self, boxes: &[Aabb], first: usize, count: usize) -> usize {
        let items = &mut self.order[first..first + count];
        let mut bounds = boxes[items[0]];
        let mut centres = Aabb::point(bounds.centre());
        for &item in items.iter() {
            bounds.merge(&boxes[item]);
            centres.grow(boxes[item].centre());
        }
        let node = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            first,
            count,
            left: 0,
            right: 0,
        });
        if count <= LEAF_SIZE {
            return node;
        }
        let extent = centres.max - centres.min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        items.sort_by(|a, b| {
            boxes[*a]
                .centre()
                .axis(axis)
                .total_cmp(&boxes[*b].centre().axis(axis))
                .then(a.cmp(b))
        });
        let half = count / 2;
        let left = self.split(boxes, first, half);
        let right = self.split(boxes, first + half, count - half);
        let inner = &mut self.nodes[node];
        inner.count = 0;
        inner.left = left;
        inner.right = right;
        node
    }

    /// Nearest hit among the members: `(distance, normal, shape index)`.
    pub fn intersect(
        &self,
        shapes: &[Shape],
        origin: Vec3,
        direction: Vec3,
        limit: f64,
    ) -> Option<(f64, Vec3, usize)> {
        if self.nodes.is_empty() {
            return None;
        }
        let inverse = Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);
        let mut best: Option<(f64, Vec3, usize)> = None;
        let mut stack = vec![0_usize];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            let reach = best.map(|hit| hit.0).unwrap_or(limit);
            if !node.bounds.hit(origin, inverse, reach) {
                continue;
            }
            if node.count > 0 {
                for &member in &self.order[node.first..node.first + node.count] {
                    if let Some((distance, normal)) = shapes[member].intersect(origin, direction) {
                        let better = match best {
                            Some((current, _, current_index)) => {
                                distance < current
                                    || (distance == current && member < current_index)
                            }
                            None => distance < limit,
                        };
                        if better {
                            best = Some((distance, normal, member));
                        }
                    }
                }
            } else {
                stack.push(node.right);
                stack.push(node.left);
            }
        }
        best
    }
}
