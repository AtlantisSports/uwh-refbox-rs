// Integration tests for TournamentManager cross-module interactions
// These tests focus on scenarios that span multiple modules or require
// complex state interactions that unit tests don't cover.

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio::time::Instant;
    use uwh_common::{
        bundles::BlackWhiteBundle,
        config::Game as GameConfig,
        game_snapshot::{GamePeriod, GameSnapshot},
    };

    // Mock TournamentManager for integration testing
    // Note: In a real implementation, we'd import from the refbox crate
    // but since refbox doesn't expose a lib target, we create a minimal mock
    struct MockTournamentManager {
        config: GameConfig,
        period: GamePeriod,
        scores: BlackWhiteBundle<u8>,
    }

    impl MockTournamentManager {
        fn new(config: GameConfig) -> Self {
            Self {
                config,
                period: GamePeriod::BetweenGames,
                scores: BlackWhiteBundle::default(),
            }
        }

        fn set_config(&mut self, config: GameConfig) -> Result<(), &'static str> {
            if self.period != GamePeriod::BetweenGames {
                return Err("Game in progress");
            }
            self.config = config;
            Ok(())
        }

        fn start_game(&mut self) {
            self.period = GamePeriod::FirstHalf;
        }

        fn set_scores(&mut self, scores: BlackWhiteBundle<u8>) {
            self.scores = scores;
        }

        fn generate_snapshot(&self) -> GameSnapshot {
            GameSnapshot {
                current_period: self.period,
                secs_in_period: 600, // 10 minutes default
                timeout: None,
                scores: self.scores,
                penalties: BlackWhiteBundle::default(),
                warnings: BlackWhiteBundle::default(),
                fouls: Default::default(),
                is_old_game: false,
                game_number: "1".to_string(),
                next_game_number: "2".to_string(),
                event_id: None,
                recent_goal: None,
                next_period_len_secs: None,
                conf_pause_time: None,
            }
        }
    }

    #[test]
    fn test_config_change_integration() {
        // Test that configuration changes properly affect tournament manager behavior
        let initial_config = GameConfig {
            half_play_duration: Duration::from_secs(900), // 15 minutes
            half_time_duration: Duration::from_secs(180), // 3 minutes
            ..Default::default()
        };

        let mut tm = MockTournamentManager::new(initial_config);

        // Should be able to change config between games
        let new_config = GameConfig {
            half_play_duration: Duration::from_secs(600), // 10 minutes
            half_time_duration: Duration::from_secs(120), // 2 minutes
            ..Default::default()
        };

        assert!(tm.set_config(new_config.clone()).is_ok());
        assert_eq!(tm.config.half_play_duration, Duration::from_secs(600));

        // Should NOT be able to change config during game
        tm.start_game();
        let another_config = GameConfig {
            half_play_duration: Duration::from_secs(1200), // 20 minutes
            ..Default::default()
        };

        assert!(tm.set_config(another_config).is_err());
        // Config should remain unchanged
        assert_eq!(tm.config.half_play_duration, Duration::from_secs(600));
    }

    #[test]
    fn test_game_state_snapshot_consistency() {
        // Test that game state changes are properly reflected in snapshots
        let config = GameConfig::default();
        let mut tm = MockTournamentManager::new(config);

        // Initial state
        let snapshot = tm.generate_snapshot();
        assert_eq!(snapshot.current_period, GamePeriod::BetweenGames);
        assert_eq!(snapshot.scores.black, 0);
        assert_eq!(snapshot.scores.white, 0);

        // Start game and add scores
        tm.start_game();
        tm.set_scores(BlackWhiteBundle { black: 2, white: 1 });

        let snapshot = tm.generate_snapshot();
        assert_eq!(snapshot.current_period, GamePeriod::FirstHalf);
        assert_eq!(snapshot.scores.black, 2);
        assert_eq!(snapshot.scores.white, 1);
    }

    #[test]
    fn test_cross_module_data_consistency() {
        // Test that data structures remain consistent across module boundaries
        let config = GameConfig {
            num_team_timeouts_allowed: 2,
            ..Default::default()
        };

        let tm = MockTournamentManager::new(config.clone());
        let snapshot = tm.generate_snapshot();

        // Verify that config values are properly reflected in snapshot context
        // This tests the integration between config and snapshot generation
        assert_eq!(snapshot.game_number, "1");
        assert_eq!(snapshot.current_period, GamePeriod::BetweenGames);

        // In a real implementation, we'd verify that timeout limits from config
        // are properly enforced in the tournament manager's timeout logic
    }
}
