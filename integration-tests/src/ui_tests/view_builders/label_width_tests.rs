#[cfg(test)]
mod tests {
    use iced_core::Length;

    // Placeholder: once helpers exist, import from utils
    const GAME_LABEL_WIDTH: f32 = 100.0;
    const REF_LABEL_WIDTH: f32 = 120.0;

    fn determine_label_width(label: &str, label_portion: u16) -> Length {
        match label {
            "Last Game" | "Next Game" => Length::Fixed(GAME_LABEL_WIDTH),
            "Chief Ref" | "Timer" | "Water Ref 1" | "Water Ref 2" | "Water Ref 3" => Length::Fixed(REF_LABEL_WIDTH),
            _ => Length::FillPortion(label_portion),
        }
    }

    #[test]
    fn test_label_width_logic() {
        let label_portion = 3; // default in app for names
        let cases = vec![
            ("Last Game", Some(GAME_LABEL_WIDTH)),
            ("Next Game", Some(GAME_LABEL_WIDTH)),
            ("Chief Ref", Some(REF_LABEL_WIDTH)),
            ("Timer", Some(REF_LABEL_WIDTH)),
            ("Water Ref 1", Some(REF_LABEL_WIDTH)),
            ("Water Ref 2", Some(REF_LABEL_WIDTH)),
            ("Water Ref 3", Some(REF_LABEL_WIDTH)),
            ("Other", None),
        ];

        for (label, expected) in cases {
            let width = determine_label_width(label, label_portion);
            match expected {
                Some(w) => match width { Length::Fixed(x) => assert!((x - w).abs() < f32::EPSILON), _ => panic!("expected fixed")},
                None => assert!(matches!(width, Length::FillPortion(_))),
            }
        }
    }
}

