//! Upload immutable geometry for a whole paint call before submitting any draw.
use egui::{
    ClippedPrimitive,
    epaint::{Primitive, Vertex},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DrawRange {
    pub index_offset_bytes: i32,
    pub index_count: i32,
}

#[derive(Default)]
pub(super) struct MeshBatch {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub ranges: Vec<DrawRange>,
}

fn valid_sizes(vertices: usize, indices: usize) -> bool {
    vertices <= u32::MAX as usize
        && vertices
            .checked_mul(std::mem::size_of::<Vertex>())
            .is_some_and(|n| n <= i32::MAX as usize)
        && indices
            .checked_mul(std::mem::size_of::<u32>())
            .is_some_and(|n| n <= i32::MAX as usize)
}

impl MeshBatch {
    /// Keep upstream's separate-mesh path for geometry beyond the GL size API.
    pub fn rebuild(&mut self, primitives: &[ClippedPrimitive]) -> bool {
        self.vertices.clear();
        self.indices.clear();
        self.ranges.clear();
        let mut vertices = 0usize;
        let mut indices = 0usize;
        for primitive in primitives {
            if let Primitive::Mesh(mesh) = &primitive.primitive {
                let Some(v) = vertices.checked_add(mesh.vertices.len()) else {
                    return false;
                };
                let Some(i) = indices.checked_add(mesh.indices.len()) else {
                    return false;
                };
                if !valid_sizes(v, i) {
                    return false;
                }
                vertices = v;
                indices = i;
            }
        }
        self.vertices.reserve(vertices);
        self.indices.reserve(indices);
        for primitive in primitives {
            if let Primitive::Mesh(mesh) = &primitive.primitive {
                debug_assert!(mesh.is_valid());
                let base = self.vertices.len() as u32;
                self.ranges.push(DrawRange {
                    index_offset_bytes: (self.indices.len() * std::mem::size_of::<u32>()) as i32,
                    index_count: mesh.indices.len() as i32,
                });
                self.vertices.extend_from_slice(&mesh.vertices);
                self.indices
                    .extend(mesh.indices.iter().map(|index| index + base));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, Mesh, Pos2, Rect, TextureId};
    fn quad(id: u64, x: f32) -> ClippedPrimitive {
        let mut mesh = Mesh::with_texture(TextureId::Managed(id));
        mesh.add_rect_with_uv(
            Rect::from_min_size(Pos2::new(x, 0.), egui::vec2(10., 10.)),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
            Color32::WHITE,
        );
        ClippedPrimitive {
            clip_rect: Rect::EVERYTHING,
            primitive: Primitive::Mesh(mesh),
        }
    }
    #[test]
    fn frame_geometry_rebases_indices_across_texture_changes_and_callbacks() {
        let primitives = vec![
            quad(1, 0.),
            ClippedPrimitive {
                clip_rect: Rect::EVERYTHING,
                primitive: Primitive::Callback(egui::PaintCallback {
                    rect: Rect::EVERYTHING,
                    callback: std::sync::Arc::new(()),
                }),
            },
            quad(2, 20.),
        ];
        let mut batch = MeshBatch::default();
        assert!(batch.rebuild(&primitives));
        assert_eq!(batch.vertices.len(), 8);
        assert_eq!(batch.ranges.len(), 2);
        assert_eq!(
            batch.ranges[0],
            DrawRange {
                index_offset_bytes: 0,
                index_count: 6
            }
        );
        assert_eq!(
            batch.ranges[1],
            DrawRange {
                index_offset_bytes: 24,
                index_count: 6
            }
        );
        assert_eq!(
            batch.indices[..6],
            match &primitives[0].primitive {
                Primitive::Mesh(m) => m.indices.clone(),
                _ => unreachable!(),
            }
        );
        assert!(batch.indices[6..].iter().all(|i| *i >= 4 && *i < 8));
        assert_eq!(batch.vertices[4].pos.x, 20.);
        assert!(batch.rebuild(&[quad(3, 40.)]));
        assert_eq!(batch.ranges.len(), 1);
        assert_eq!(batch.indices.len(), 6);
        assert!(batch.indices.iter().all(|i| *i < 4));
        assert_eq!(batch.vertices[0].pos.x, 40.);
    }
    #[test]
    fn frame_geometry_rejects_size_overflow_and_clears_empty_frames() {
        assert!(!valid_sizes(usize::MAX, 1));
        assert!(!valid_sizes(1, usize::MAX));
        assert!(!valid_sizes(1, i32::MAX as usize / 4 + 1));
        let mut batch = MeshBatch::default();
        assert!(batch.rebuild(&[quad(0, 0.)]));
        assert!(batch.rebuild(&[]));
        assert!(batch.vertices.is_empty());
        assert!(batch.indices.is_empty());
        assert!(batch.ranges.is_empty());
    }
}
