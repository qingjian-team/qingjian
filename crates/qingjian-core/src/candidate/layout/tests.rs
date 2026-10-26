//! 自动末尾布局与显式置顶布局的分页、去重及固定短语回归。

mod automatic {
    use crate::candidate::layout::{CandidateLayout, Cell};
    use crate::{Candidate, CandidateKind};

    fn local(text: &str) -> Candidate {
        Candidate {
            text: text.into(),
            kind: CandidateKind::Chinese,
            syllables: vec!["zhang".into(), "tao".into()],
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    fn cloud(text: &str) -> Candidate {
        Candidate {
            kind: CandidateKind::Cloud,
            ..local(text)
        }
    }

    fn texts(cells: &[Cell<'_>]) -> Vec<String> {
        cells
            .iter()
            .map(|cell| match cell {
                Cell::Local(c) => c.text.clone(),
                Cell::Cloud(c) => format!("☁{}", c.text),
                Cell::Empty => "<empty>".into(),
            })
            .collect()
    }

    fn many(count: usize) -> Vec<Candidate> {
        (0..count).map(|i| local(&format!("本{i}"))).collect()
    }

    #[test]
    fn cloud_words_take_the_end_of_the_first_page_and_only_bump_those_cells() {
        let mut layout = CandidateLayout::new(many(12), 9, 3);
        assert_eq!(
            texts(&layout.page(0)),
            [
                "本0", "本1", "本2", "本3", "本4", "本5", "本6", "本7", "本8"
            ]
        );
        assert_eq!(layout.pages(), 2);

        // 四条云端词只取前三条；前六格不动，原来 7–9 格的本地候选挪到第二页开头
        let filled = layout.set_cloud(vec![cloud("云0"), cloud("云1"), cloud("云2"), cloud("云3")]);
        assert_eq!(filled, 3);
        assert_eq!(
            texts(&layout.page(0)),
            [
                "本0", "本1", "本2", "本3", "本4", "本5", "☁云0", "☁云1", "☁云2"
            ]
        );
        assert_eq!(
            texts(&layout.page(1)),
            ["本6", "本7", "本8", "本9", "本10", "本11"]
        );
        assert_eq!(layout.candidate(6).unwrap().text, "云0");
        assert_eq!(layout.candidate(9).unwrap().text, "本6");
    }

    #[test]
    fn duplicates_of_local_candidates_are_dropped_and_fewer_words_take_fewer_cells() {
        let mut layout = CandidateLayout::new(many(4), 9, 2);
        assert_eq!(layout.set_cloud(vec![cloud("本2"), cloud("云0")]), 1);
        assert_eq!(texts(&layout.page(0)), ["本0", "本1", "本2", "本3", "☁云0"]);
        assert_eq!(layout.local()[2].kind, CandidateKind::Chinese);
        assert_eq!(layout.pages(), 1);
    }

    #[test]
    fn no_words_means_nothing_changes() {
        let mut layout = CandidateLayout::new(many(12), 9, 2);
        assert_eq!(layout.set_cloud(Vec::new()), 0);
        assert_eq!(layout.page(0).len(), 9);
        assert_eq!(texts(&layout.page(1)), ["本9", "本10", "本11"]);
    }

    #[test]
    fn zero_slots_means_no_cloud_cells_at_all() {
        let mut layout = CandidateLayout::new(many(3), 9, 0);
        assert_eq!(layout.set_cloud(vec![cloud("云0")]), 0);
        assert_eq!(texts(&layout.page(0)), ["本0", "本1", "本2"]);
    }

    #[test]
    fn without_local_candidates_the_whole_page_goes_to_the_cloud() {
        let mut layout = CandidateLayout::new(Vec::new(), 9, 2);
        assert_eq!(
            layout.set_cloud(vec![cloud("森"), cloud("淼"), cloud("焱")]),
            3
        );
        assert_eq!(texts(&layout.page(0)), ["☁森", "☁淼", "☁焱"]);
        assert_eq!(layout.candidate(0).unwrap().text, "森");
    }

    #[test]
    fn slots_never_push_the_first_local_candidate_off_the_first_page() {
        let mut layout = CandidateLayout::new(many(5), 3, 5);
        assert_eq!(layout.capacity(), 2);
        layout.set_cloud(vec![cloud("云0"), cloud("云1"), cloud("云2")]);
        assert_eq!(texts(&layout.page(0)), ["本0", "☁云0", "☁云1"]);
        assert_eq!(texts(&layout.page(1)), ["本1", "本2", "本3"]);
    }

    #[test]
    fn sparse_custom_positions_are_layout_cells_not_candidates() {
        let fixed = Candidate {
            kind: CandidateKind::Custom(3),
            ..local("短语")
        };
        let mut layout = CandidateLayout::new(vec![fixed, local("普通")], 5, 2);
        assert_eq!(layout.local().len(), 2);
        assert_eq!(texts(&layout.page(0)), ["普通", "<empty>", "短语"]);
        assert!(layout.candidate(1).is_none());
        assert_eq!(layout.set_cloud(vec![cloud("云")]), 1);
        assert_eq!(texts(&layout.page(0)), ["普通", "<empty>", "短语", "☁云"]);
    }

    #[test]
    fn ninth_position_stays_on_second_page_without_cloud_displacement() {
        let mut layout = CandidateLayout::new(
            vec![Candidate {
                kind: CandidateKind::Custom(9),
                ..local("第九")
            }],
            5,
            2,
        );
        assert_eq!(layout.pages(), 2);
        assert_eq!(layout.capacity(), 0);
        assert_eq!(layout.set_cloud(vec![cloud("云")]), 0);
        assert_eq!(layout.candidate(8).unwrap().text, "第九");
        assert!(layout.candidate(7).is_none());
    }
}

mod manual {
    use crate::candidate::layout::{CandidateLayout, Cell};
    use crate::{Candidate, CandidateKind};

    fn local(text: &str) -> Candidate {
        Candidate {
            text: text.into(),
            kind: CandidateKind::Chinese,
            syllables: vec!["zhang".into(), "tao".into()],
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    fn cloud(text: &str) -> Candidate {
        Candidate {
            kind: CandidateKind::Cloud,
            ..local(text)
        }
    }

    fn texts(cells: &[Cell<'_>]) -> Vec<String> {
        cells
            .iter()
            .map(|cell| match cell {
                Cell::Local(c) => c.text.clone(),
                Cell::Cloud(c) => format!("☁{}", c.text),
                Cell::Empty => "<empty>".into(),
            })
            .collect()
    }

    fn many(count: usize) -> Vec<Candidate> {
        (0..count).map(|i| local(&format!("本{i}"))).collect()
    }

    #[test]
    fn cloud_words_take_the_front_and_local_candidates_keep_their_order() {
        let mut layout = CandidateLayout::new(many(12), 9, 3);
        assert_eq!(
            texts(&layout.page(0)),
            [
                "本0", "本1", "本2", "本3", "本4", "本5", "本6", "本7", "本8"
            ]
        );
        assert_eq!(layout.pages(), 2);

        // 四条云端词只取前三条，原本的本地候选按原顺序接在后面。
        let filled =
            layout.set_cloud_first(vec![cloud("云0"), cloud("云1"), cloud("云2"), cloud("云3")]);
        assert_eq!(filled, 3);
        assert_eq!(
            texts(&layout.page(0)),
            [
                "☁云0", "☁云1", "☁云2", "本0", "本1", "本2", "本3", "本4", "本5"
            ]
        );
        assert_eq!(
            texts(&layout.page(1)),
            ["本6", "本7", "本8", "本9", "本10", "本11"]
        );
        assert_eq!(layout.candidate(0).unwrap().text, "云0");
        assert_eq!(layout.candidate(9).unwrap().text, "本6");
    }

    #[test]
    fn local_duplicates_are_promoted_and_restored_when_cloud_is_cleared() {
        let mut layout = CandidateLayout::new(many(4), 9, 2);
        assert_eq!(
            layout.set_cloud_first(vec![cloud("本2"), cloud("本2"), cloud("云0")]),
            2
        );
        assert_eq!(
            texts(&layout.page(0)),
            ["☁本2", "☁云0", "本0", "本1", "本3"]
        );
        assert_eq!(layout.len(), 5);
        assert_eq!(layout.local()[2].kind, CandidateKind::Chinese);
        assert_eq!(layout.pages(), 1);
        layout.set_cloud_first(Vec::new());
        assert_eq!(texts(&layout.page(0)), ["本0", "本1", "本2", "本3"]);
    }

    #[test]
    fn no_words_means_nothing_changes() {
        let mut layout = CandidateLayout::new(many(12), 9, 2);
        assert_eq!(layout.set_cloud_first(Vec::new()), 0);
        assert_eq!(layout.page(0).len(), 9);
        assert_eq!(texts(&layout.page(1)), ["本9", "本10", "本11"]);
    }

    #[test]
    fn zero_slots_means_no_cloud_cells_at_all() {
        let mut layout = CandidateLayout::new(many(3), 9, 0);
        assert_eq!(layout.set_cloud_first(vec![cloud("云0")]), 0);
        assert_eq!(texts(&layout.page(0)), ["本0", "本1", "本2"]);
    }

    #[test]
    fn without_local_candidates_the_whole_page_goes_to_the_cloud() {
        let mut layout = CandidateLayout::new(Vec::new(), 9, 2);
        assert_eq!(
            layout.set_cloud_first(vec![cloud("森"), cloud("淼"), cloud("焱")]),
            3
        );
        assert_eq!(texts(&layout.page(0)), ["☁森", "☁淼", "☁焱"]);
        assert_eq!(layout.candidate(0).unwrap().text, "森");
    }

    #[test]
    fn cloud_can_fill_the_first_page_and_local_candidates_follow() {
        let mut layout = CandidateLayout::new(many(5), 3, 5);
        layout.set_cloud_first(vec![cloud("云0"), cloud("云1"), cloud("云2")]);
        assert_eq!(layout.capacity(), 3);
        assert_eq!(texts(&layout.page(0)), ["☁云0", "☁云1", "☁云2"]);
        assert_eq!(texts(&layout.page(1)), ["本0", "本1", "本2"]);
        let mut single = CandidateLayout::new(many(1), 1, 1);
        assert_eq!(single.set_cloud_first(vec![cloud("云")]), 1);
        assert_eq!(texts(&single.page(0)), ["☁云"]);
    }

    #[test]
    fn sparse_custom_positions_are_layout_cells_not_candidates() {
        let fixed = Candidate {
            kind: CandidateKind::Custom(3),
            ..local("短语")
        };
        let mut layout = CandidateLayout::new(vec![fixed, local("普通")], 5, 2);
        assert_eq!(layout.local().len(), 2);
        assert_eq!(texts(&layout.page(0)), ["普通", "<empty>", "短语"]);
        assert!(layout.candidate(1).is_none());
        assert_eq!(layout.set_cloud_first(vec![cloud("云")]), 1);
        assert_eq!(texts(&layout.page(0)), ["☁云", "普通", "短语"]);
        assert_eq!(layout.set_cloud_first(vec![cloud("短语"), cloud("云")]), 1);
        assert_eq!(layout.candidate(2).unwrap().text, "短语");
    }

    #[test]
    fn ninth_position_stays_on_second_page_without_cloud_displacement() {
        let mut layout = CandidateLayout::new(
            vec![Candidate {
                kind: CandidateKind::Custom(9),
                ..local("第九")
            }],
            5,
            2,
        );
        assert_eq!(layout.pages(), 2);
        assert_eq!(layout.capacity(), 0);
        assert_eq!(layout.set_cloud_first(vec![cloud("云")]), 1);
        assert_eq!(layout.capacity(), 2);
        assert_eq!(layout.candidate(8).unwrap().text, "第九");
        assert!(layout.candidate(7).is_none());
    }

    #[test]
    fn legacy_entry_keeps_tail_slots_and_drops_local_duplicates() {
        let mut layout = CandidateLayout::new(many(4), 9, 2);
        layout.set_cloud_first(vec![cloud("本2"), cloud("云")]);
        assert_eq!(layout.set_cloud(vec![cloud("本2"), cloud("云")]), 1);
        assert_eq!(texts(&layout.page(0)), ["本0", "本1", "本2", "本3", "☁云"]);
    }

    #[test]
    fn legacy_entry_reserves_local_first_and_sparse_fixed_positions() {
        let mut single = CandidateLayout::new(many(1), 1, 1);
        assert_eq!(single.set_cloud(vec![cloud("云")]), 0);
        let mut fixed = CandidateLayout::new(
            vec![Candidate {
                kind: CandidateKind::Custom(9),
                ..local("第九")
            }],
            5,
            2,
        );
        assert_eq!(fixed.set_cloud(vec![cloud("云")]), 0);
        assert_eq!(fixed.candidate(8).unwrap().text, "第九");
    }
}
