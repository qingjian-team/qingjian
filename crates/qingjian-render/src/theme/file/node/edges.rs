//! 四边的值：一个数四边相同，或 `[上, 右, 下, 左]`，与 CSS 同序。

use serde::Deserialize;

use super::length::Length;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(from = "EdgesRepr")]
pub(crate) struct Edges {
    pub(crate) top: Length,

    pub(crate) right: Length,

    pub(crate) bottom: Length,

    pub(crate) left: Length,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EdgesRepr {
    All(Length),

    Each([Length; 4]),
}

impl From<EdgesRepr> for Edges {
    fn from(value: EdgesRepr) -> Self {
        let [top, right, bottom, left] = match value {
            EdgesRepr::All(length) => [length; 4],
            EdgesRepr::Each(edges) => edges,
        };
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::length::Length;
    use super::Edges;

    #[test]
    fn a_single_number_repeats_on_all_four_sides() {
        let edges: Edges = serde_json::from_str("6").unwrap();
        for side in [edges.top, edges.right, edges.bottom, edges.left] {
            assert_eq!(side, Length::Points(6.0));
        }
        let auto: Edges = serde_json::from_str("\"auto\"").unwrap();
        assert_eq!(auto.top, Length::Auto);
        assert_eq!(auto.left, Length::Auto);
    }

    #[test]
    fn an_array_maps_to_top_right_bottom_left() {
        let edges: Edges = serde_json::from_str("[1, 2, \"auto\", 4]").unwrap();
        assert_eq!(
            edges,
            Edges {
                top: Length::Points(1.0),
                right: Length::Points(2.0),
                bottom: Length::Auto,
                left: Length::Points(4.0),
            }
        );
    }

    #[test]
    fn wrong_shapes_are_rejected() {
        assert!(serde_json::from_str::<Edges>("[1, 2, 3]").is_err());
        assert!(serde_json::from_str::<Edges>("[1, 2, 3, 4, 5]").is_err());
        assert!(serde_json::from_str::<Edges>("\"thin\"").is_err());
        assert!(serde_json::from_str::<Edges>("null").is_err());
        assert_eq!(
            serde_json::from_str::<Edges>("\"auto\" ").unwrap().top,
            Length::Auto
        );
    }
}
