#[cfg(test)]
mod tests {
    use crate::utils::layout_assertions::{assert_mock_layout_structure, determine_label_width,
        GAME_LABEL_WIDTH, REF_LABEL_WIDTH, MockRow};
    use iced_core::Length;

    #[test]
    fn test_structure_and_widths() {
        // Build a minimal set of rows similar to app
        let rows = vec![
            MockRow{ left_label: "Last Game".into(), left_value: "White".into(), center_label: Some("Team A".into()), center_value: None },
            MockRow{ left_label: "".into(), left_value: "Black".into(), center_label: Some("Team B".into()), center_value: None },
            MockRow{ left_label: "Next Game".into(), left_value: "White".into(), center_label: Some("Team C".into()), center_value: None },
            MockRow{ left_label: "".into(), left_value: "Black".into(), center_label: Some("Team D".into()), center_value: None },
            MockRow{ left_label: "Chief Ref".into(), left_value: "Unknown".into(), center_label: None, center_value: None },
            MockRow{ left_label: "Timer".into(), left_value: "Unknown".into(), center_label: None, center_value: None },
            MockRow{ left_label: "Water Ref 1".into(), left_value: "Unknown".into(), center_label: None, center_value: None },
            MockRow{ left_label: "Water Ref 2".into(), left_value: "Unknown".into(), center_label: None, center_value: None },
            MockRow{ left_label: "Water Ref 3".into(), left_value: "Unknown".into(), center_label: None, center_value: None },
        ];

        assert_mock_layout_structure(&rows);

        let lp = 3u16;
        // Verify widths for specific labels
        assert!(matches!(determine_label_width("Last Game", lp), Length::Fixed(w) if (w - GAME_LABEL_WIDTH).abs() < f32::EPSILON));
        assert!(matches!(determine_label_width("", lp), Length::Fixed(w) if (w - GAME_LABEL_WIDTH).abs() < f32::EPSILON));
        for lbl in ["Chief Ref","Timer","Water Ref 1","Water Ref 2","Water Ref 3"] {
            assert!(matches!(determine_label_width(lbl, lp), Length::Fixed(w) if (w - REF_LABEL_WIDTH).abs() < f32::EPSILON));
        }
        assert!(matches!(determine_label_width("Other", lp), Length::FillPortion(p) if p == lp));
    }
}
