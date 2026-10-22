//! Comparación de las posiciones generadas por dos rasterizaciones.

use crate::fragment::Fragment;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    pub count_a: usize,
    pub count_b: usize,
    /// Posiciones que solo genera A / solo genera B.
    pub only_in_a: Vec<(i32, i32)>,
    pub only_in_b: Vec<(i32, i32)>,
    /// Fragmentos repetidos (misma posición más de una vez) en cada resultado.
    pub duplicates_a: usize,
    pub duplicates_b: usize,
}

impl Comparison {
    /// `true` si ambos generan exactamente el mismo multiconjunto de posiciones.
    pub fn identical(&self) -> bool {
        self.count_a == self.count_b
            && self.only_in_a.is_empty()
            && self.only_in_b.is_empty()
            && self.duplicates_a == self.duplicates_b
    }
}

fn sorted_positions(fragments: &[Fragment]) -> Vec<(i32, i32)> {
    let mut positions: Vec<(i32, i32)> = fragments.iter().map(|f| (f.y, f.x)).collect();
    positions.sort_unstable();
    positions
}

fn count_duplicates(sorted: &[(i32, i32)]) -> usize {
    sorted.windows(2).filter(|w| w[0] == w[1]).count()
}

/// Compara posiciones (no colores) de dos listas de fragmentos.
pub fn compare_fragments(a: &[Fragment], b: &[Fragment]) -> Comparison {
    let pa = sorted_positions(a);
    let pb = sorted_positions(b);

    let mut only_in_a = Vec::new();
    let mut only_in_b = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < pa.len() || j < pb.len() {
        match (pa.get(i), pb.get(j)) {
            (Some(x), Some(y)) if x == y => {
                i += 1;
                j += 1;
            }
            (Some(x), Some(y)) if x < y => {
                only_in_a.push((x.1, x.0));
                i += 1;
            }
            (Some(_), Some(y)) => {
                only_in_b.push((y.1, y.0));
                j += 1;
            }
            (Some(x), None) => {
                only_in_a.push((x.1, x.0));
                i += 1;
            }
            (None, Some(y)) => {
                only_in_b.push((y.1, y.0));
                j += 1;
            }
            (None, None) => unreachable!(),
        }
    }

    Comparison {
        count_a: a.len(),
        count_b: b.len(),
        only_in_a,
        only_in_b,
        duplicates_a: count_duplicates(&pa),
        duplicates_b: count_duplicates(&pb),
    }
}
